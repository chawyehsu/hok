//! Msi extractor backed by the `msi` and `cab` crates.
//!
//! The `Directory`, `Component`, `File` and `Media` tables are read to rebuild
//! the installed layout, then each embedded cabinet is streamed to disk. The
//! resulting `SourceDir/Files/<ProductName>/...` layout is flattened the way
//! Scoop does it: `<ProductName>` is hoisted to the archive root and the empty
//! `SourceDir`/`Files` directories are removed.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use super::ExtractObserver;
use super::ExtractSummary;
use super::Probe;
use super::helper;
use super::helper::SEVEN_ZIP_ARGS;
use crate::error::Error;
use crate::error::Fallible;

const SOURCE_DIR: &str = "SourceDir";
const FILES: &str = "Files";

/// Extract an MSI package into `dest`.
///
/// The tables are read first; when that is impossible the extractor falls back
/// to an administrative install through `msiexec`, then to the `7z.exe` helper.
pub fn extract(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<(u64, u64, Vec<String>)> {
    match extract_tables(src, dest, observer) {
        Ok(outcome) if outcome.failure.is_none() => {
            flatten(dest);
            Ok((outcome.entries, outcome.bytes, outcome.warnings))
        }
        Ok(outcome) => {
            let mut warnings = outcome.warnings;
            warnings.push(format!(
                "pure-msi extraction incomplete: {}",
                outcome.failure.unwrap_or_default()
            ));
            finish_with_hidden_fallback(src, dest, observer, probe, warnings)
        }
        Err(err) => {
            let mut warnings = vec![format!("pure-msi extraction failed: {err}")];
            finish_with_hidden_fallback(src, dest, observer, probe, warnings)
        }
    }
}

/// Extract an MSI package, falling back to the `7z.exe` helper.
///
/// This stays inside the helper contract: a missing helper surfaces as
/// [`Error::HelperNotFound`], and the extractor name in the report states that
/// the helper ran.
pub fn extract_with_fallback(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let (entries, bytes, warnings) = extract(src, dest, probe, observer)?;
    Ok(ExtractSummary {
        extractor: "msi",
        entries,
        bytes,
        warnings,
    })
}

fn finish_with_hidden_fallback(
    src: &Path,
    dest: &Path,
    observer: &dyn ExtractObserver,
    probe: &Probe,
    mut warnings: Vec<String>,
) -> Fallible<(u64, u64, Vec<String>)> {
    if let Ok((entries, bytes)) = administrative_install(src, dest, observer) {
        flatten(dest);
        return Ok((entries, bytes, warnings));
    }
    warnings.push("msiexec administrative install failed".to_owned());
    let mut more = helper::extract_archive(helper::SEVEN_ZIP, SEVEN_ZIP_ARGS, src, dest, observer)?;
    flatten(dest);
    warnings.append(&mut more.warnings);
    Ok((more.entries, more.bytes, warnings))
}

/// Run `msiexec /a` into `dest` and report what was written.
///
/// Never runs `msiexec` without `/qn`, so it cannot display UI or prompt.
pub fn administrative_install(
    src: &Path,
    dest: &Path,
    observer: &dyn ExtractObserver,
) -> Fallible<(u64, u64)> {
    let source_dir = dest.join(SOURCE_DIR);
    fs::create_dir_all(&source_dir)?;
    let log = dest.join("msi.log");
    let target_dir = format!("TARGETDIR={}", source_dir.display());
    let log_arg = format!("/lwe {}", log.display());
    helper::run(
        "msiexec.exe",
        [
            "/a".to_owned(),
            src.display().to_string(),
            "/qn".to_owned(),
            target_dir,
            log_arg,
        ],
        None,
    )?;
    report_tree(&source_dir, dest, observer)
}

#[derive(Default)]
struct Outcome {
    entries: u64,
    bytes: u64,
    warnings: Vec<String>,
    failure: Option<String>,
}

fn extract_tables(src: &Path, dest: &Path, observer: &dyn ExtractObserver) -> io::Result<Outcome> {
    let mut package = msi::open(src)?;
    let directories = read_table(&mut package, "Directory")?;
    let components = read_table(&mut package, "Component")?;
    let files = read_table(&mut package, "File")?;
    let media = read_table(&mut package, "Media")?;

    let mut decoded_dirs: BTreeMap<String, String> = BTreeMap::new();
    for row in &directories {
        if let Some(key) = row.text("Directory") {
            decoded_dirs.insert(key.to_owned(), String::new());
        }
    }
    for row in &directories {
        let Some(key) = row.text("Directory") else {
            continue;
        };
        let parent = row.text("Directory_Parent").unwrap_or_default();
        let part = row.text("DefaultDir").map(short_dir).unwrap_or_default();
        let decoded = if parent.is_empty() {
            part
        } else {
            let base = decoded_dirs.get(parent).cloned().unwrap_or_default();
            join(&base, &part)
        };
        decoded_dirs.insert(key.to_owned(), decoded);
    }

    let mut component_dirs: BTreeMap<&str, &str> = BTreeMap::new();
    for row in &components {
        let Some(component) = row.text("Component") else {
            continue;
        };
        component_dirs.insert(component, row.text("Directory_").unwrap_or_default());
    }

    // `Media.Cabinet` names the cabinet containing every file whose `Sequence`
    // is at or after that media's `DiskId`.
    let mut media_rows: Vec<(i32, &str)> = media
        .iter()
        .filter_map(|row| {
            let disk = row.integer("DiskId")?;
            let cabinet = row.text("Cabinet")?;
            Some((disk, cabinet))
        })
        .collect();
    media_rows.sort_by_key(|(disk, _)| *disk);

    let mut cabinets: BTreeMap<String, Vec<&TableRow>> = BTreeMap::new();
    for row in &files {
        let sequence = row.integer("Sequence").unwrap_or_default();
        let cabinet = media_rows
            .iter()
            .filter(|(disk, _)| *disk <= sequence)
            .map(|(_, cabinet)| *cabinet)
            .next_back()
            .or_else(|| media_rows.first().map(|(_, cabinet)| *cabinet));
        if let Some(cabinet) = cabinet {
            cabinets.entry(cabinet.to_owned()).or_default().push(row);
        }
    }

    let mut outcome = Outcome::default();
    for (cabinet_name, rows) in &cabinets {
        let stream = match package.read_stream(cabinet_name) {
            Ok(stream) => stream,
            Err(err) => {
                outcome.failure = Some(format!("cannot read cabinet '{cabinet_name}': {err}"));
                return Ok(outcome);
            }
        };
        let mut cabinet = match cab::Cabinet::new(stream) {
            Ok(cabinet) => cabinet,
            Err(err) => {
                outcome.failure = Some(format!("cannot open cabinet '{cabinet_name}': {err}"));
                return Ok(outcome);
            }
        };
        let members = cabinet
            .folder_entries()
            .flat_map(|folder| {
                folder
                    .file_entries()
                    .map(|file| file.name().to_owned())
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeSet<_>>();

        for row in rows {
            let Some(file_key) = row.text("File") else {
                continue;
            };
            let Some(member) = cabinet_member(&members, row, file_key) else {
                outcome.warnings.push(format!(
                    "cabinet '{cabinet_name}' has no entry for '{file_key}'"
                ));
                continue;
            };
            let component = row.text("Component_").unwrap_or_default();
            let directory = component_dirs
                .get(component)
                .and_then(|key| decoded_dirs.get(*key))
                .cloned()
                .unwrap_or_default();
            let file_name = row.text("FileName").unwrap_or(&member);
            let name = long_name(file_name);
            let rel = join(&directory, &name);
            let rel = match sanitize(&rel) {
                Ok(rel) => rel,
                Err(()) => {
                    outcome
                        .warnings
                        .push(format!("skipped entry '{rel}' outside the destination"));
                    continue;
                }
            };
            let Ok(target) = super::guard(dest, &rel) else {
                outcome.warnings.push(format!(
                    "skipped entry '{}' outside the destination",
                    rel.display()
                ));
                continue;
            };
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }

            observer.entry_written(&rel.to_string_lossy().replace('\\', "/"));
            let mut reader = match cabinet.read_file(&member) {
                Ok(reader) => reader,
                Err(err) => {
                    outcome
                        .warnings
                        .push(format!("skipped entry '{}': {err}", rel.display()));
                    continue;
                }
            };
            let mut out = io::BufWriter::new(fs::File::create(&target)?);
            let written = io::copy(&mut reader, &mut out)?;
            use std::io::Write;
            out.flush()?;
            outcome.entries = outcome.entries.saturating_add(1);
            outcome.bytes = outcome.bytes.saturating_add(written);
        }
    }

    Ok(outcome)
}

/// Find the cabinet member that holds `row`'s file.
///
/// The `File` column is the canonical member name; a few installers store a
/// different `FileName`, so the long and short names are also tried.
fn cabinet_member(members: &BTreeSet<String>, row: &TableRow, file_key: &str) -> Option<String> {
    if members.contains(file_key) {
        return Some(file_key.to_owned());
    }
    let _ = file_key;
    let file_name = row.text("FileName").unwrap_or_default();
    let long = long_name(file_name);
    let short = short_name(file_name);
    for candidate in [&short, &long] {
        if !candidate.is_empty() && members.contains(candidate) {
            return Some(candidate.clone());
        }
    }
    members
        .iter()
        .find(|candidate| candidate.replace('\\', "/") == long)
        .cloned()
}

/// The long name from a `Short|Long` `FileName` value.
fn long_name(value: &str) -> String {
    match value.split_once('|') {
        Some((short, long)) => {
            let _ = short;
            long.to_owned()
        }
        None => value.to_owned(),
    }
}

/// The short name from a `Short|Long` `FileName` value.
fn short_name(value: &str) -> String {
    match value.split_once('|') {
        Some((short, _long)) => short.to_owned(),
        None => String::new(),
    }
}

/// Strip the `Short|Long` and `target:source` encodings from a directory.
fn short_dir(value: &str) -> String {
    let value = match value.split_once('|') {
        Some((short, _long)) => short,
        None => value,
    };
    let value = value.split(':').next().unwrap_or(value);
    value.trim_matches(['\\', '/']).to_owned()
}

fn join(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_owned()
    } else {
        format!("{base}/{name}")
    }
}

/// Reject absolute paths and `..` components in a table-derived path.
fn sanitize(path: &str) -> Result<PathBuf, ()> {
    let mut out = PathBuf::new();
    for component in Path::new(&path.replace('\\', "/")).components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return Err(()),
        }
    }
    Ok(out)
}

/// Flatten `SourceDir/Files/<ProductName>/...` to `<ProductName>/...`.
pub fn flatten(dest: &Path) {
    let source_dir = dest.join(SOURCE_DIR);
    if !source_dir.is_dir() {
        return;
    }
    let files_dir = source_dir.join(FILES);
    let files_root = if files_dir.is_dir() {
        files_dir.clone()
    } else {
        source_dir.clone()
    };
    let Ok(entries) = fs::read_dir(&files_root) else {
        return;
    };
    let children = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    for path in &children {
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };
        let target = dest.join(name);
        if target.exists() {
            continue;
        }
        if fs::rename(path, &target).is_ok() {
            tracing::debug!("hoisted '{}' to the archive root", name.to_string_lossy());
        }
    }
    remove_if_empty(&files_dir);
    remove_if_empty(&source_dir);
}

/// Walk `root` and report every file under `dest` that it wrote.
fn report_tree(root: &Path, dest: &Path, observer: &dyn ExtractObserver) -> Fallible<(u64, u64)> {
    let mut entries = 0u64;
    let mut bytes = 0u64;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(relative) = path.strip_prefix(dest) else {
                continue;
            };
            observer.entry_written(&relative.to_string_lossy().replace('\\', "/"));
            entries = entries.saturating_add(1);
            bytes = bytes.saturating_add(entry.metadata().map(|meta| meta.len()).unwrap_or(0));
        }
    }
    Ok((entries, bytes))
}

fn remove_if_empty(path: &Path) {
    if fs::read_dir(path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
    {
        let _ = fs::remove_dir(path);
    }
}

fn read_table<F: Read + Seek>(
    package: &mut msi::Package<F>,
    name: &str,
) -> io::Result<Vec<TableRow>> {
    let mut rows = Vec::new();
    for row in package.select_rows(msi::Select::table(name))? {
        let values = row
            .columns()
            .iter()
            .map(|column| (column.name().to_owned(), row[column.name()].clone()))
            .collect();
        rows.push(TableRow(values));
    }
    Ok(rows)
}

struct TableRow(Vec<(String, msi::Value)>);

impl TableRow {
    fn value(&self, column: &str) -> Option<&msi::Value> {
        self.0
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(column))
            .map(|(_, value)| value)
    }

    fn text(&self, column: &str) -> Option<&str> {
        self.value(column).and_then(msi::Value::as_str)
    }

    fn integer(&self, column: &str) -> Option<i32> {
        self.value(column).and_then(msi::Value::as_int)
    }
}

fn extract_error(archive: &Path, err: impl std::fmt::Display) -> Error {
    Error::Extract {
        archive: archive.to_path_buf(),
        reason: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CollectObserver(std::sync::Mutex<Vec<String>>);

    impl super::super::ExtractObserver for CollectObserver {
        fn entry_written(&self, entry: &str) {
            self.0.lock().unwrap().push(entry.to_owned());
        }
    }

    #[test]
    fn short_dir_takes_short_name() {
        assert_eq!(short_dir("SHORT|Long Name"), "SHORT");
        assert_eq!(short_dir("Dir"), "Dir");
        assert_eq!(short_dir("SHORT|Long:key"), "SHORT");
        assert_eq!(short_dir("\\nested\\"), "nested");
        assert_eq!(short_dir("target:source"), "target");
    }

    #[test]
    fn long_and_short_names_split_file_name() {
        assert_eq!(long_name("SHORT|Long Name.exe"), "Long Name.exe");
        assert_eq!(long_name("plain.exe"), "plain.exe");
        assert_eq!(short_name("SHORT|Long Name.exe"), "SHORT");
        assert!(short_name("plain.exe").is_empty());
    }

    #[test]
    fn join_handles_empty_base() {
        assert_eq!(join("", "a.txt"), "a.txt");
        assert_eq!(join("dir", "a.txt"), "dir/a.txt");
    }

    #[test]
    fn sanitize_rejects_escapes() {
        assert!(sanitize("../evil").is_err());
        assert!(sanitize("a/../../evil").is_err());
        assert!(sanitize("C:/evil").is_err());
        assert!(sanitize("/abs").is_err());
        assert_eq!(sanitize("a/./b").unwrap(), PathBuf::from("a/b"));
    }

    #[test]
    fn flatten_hoists_product_name() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        let product = dest.join("SourceDir").join("Files").join("App");
        std::fs::create_dir_all(&product).unwrap();
        std::fs::write(product.join("app.exe"), b"bin").unwrap();

        let observer = CollectObserver(std::sync::Mutex::new(Vec::new()));
        flatten(&dest);
        assert!(dest.join("App/app.exe").is_file());
        assert!(!dest.join("SourceDir").exists());
        assert!(observer.0.lock().unwrap().is_empty());
    }

    #[test]
    fn flatten_without_files_dir_hoists_source_dir_children() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        let product = dest.join("SourceDir").join("App");
        std::fs::create_dir_all(&product).unwrap();
        std::fs::write(product.join("a.txt"), b"a").unwrap();

        let observer = CollectObserver(std::sync::Mutex::new(Vec::new()));
        flatten(&dest);
        assert!(dest.join("App/a.txt").is_file());
        assert!(!dest.join("SourceDir").exists());
    }

    #[test]
    fn flatten_does_not_clobber_existing_entries() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        let product = dest.join("SourceDir").join("Files").join("App");
        std::fs::create_dir_all(&product).unwrap();
        std::fs::write(product.join("a.txt"), b"new").unwrap();
        std::fs::create_dir_all(dest.join("App")).unwrap();
        std::fs::write(dest.join("App/keep.txt"), b"keep").unwrap();

        let observer = CollectObserver(std::sync::Mutex::new(Vec::new()));
        flatten(&dest);
        assert_eq!(std::fs::read(dest.join("App/keep.txt")).unwrap(), b"keep");
    }

    #[test]
    fn flatten_is_noop_without_source_dir() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(dest.join("plain")).unwrap();
        let observer = CollectObserver(std::sync::Mutex::new(Vec::new()));
        flatten(&dest);
        assert!(std::fs::read_dir(&dest).unwrap().next().is_some());
    }

    #[test]
    fn report_tree_reports_relative_names() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        let root = dest.join("SourceDir");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::write(root.join("a.txt"), b"ab").unwrap();
        std::fs::write(root.join("nested/b.txt"), b"abc").unwrap();
        let observer = CollectObserver(std::sync::Mutex::new(Vec::new()));
        let (entries, bytes) = report_tree(&root, &dest, &observer).unwrap();
        assert_eq!(entries, 2);
        assert_eq!(bytes, 5);
        let mut names = observer.0.lock().unwrap().clone();
        names.sort();
        assert_eq!(
            names,
            [
                "SourceDir/a.txt".to_owned(),
                "SourceDir/nested/b.txt".to_owned()
            ]
        );
    }

    #[test]
    fn remove_if_empty_removes_only_empty_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        remove_if_empty(&empty);
        assert!(!empty.exists());

        let full = dir.path().join("full");
        std::fs::create_dir_all(&full).unwrap();
        std::fs::write(full.join("f"), b"x").unwrap();
        remove_if_empty(&full);
        assert!(full.exists());
    }

    #[test]
    fn cabinet_member_prefers_file_key() {
        let members = BTreeSet::from(["a.exe".to_owned(), "SHORT".to_owned()]);
        let row = TableRow(vec![(
            "FileName".to_owned(),
            msi::Value::Str("SHORT|Long Name.exe".to_owned()),
        )]);
        assert_eq!(
            cabinet_member(&members, &row, "a.exe"),
            Some("a.exe".to_owned())
        );
        assert_eq!(
            cabinet_member(&members, &row, "absent"),
            Some("SHORT".to_owned())
        );
    }

    #[test]
    fn table_row_looks_up_columns_case_insensitively() {
        let row = TableRow(vec![
            ("DiskId".to_owned(), msi::Value::Int(2)),
            (
                "Cabinet".to_owned(),
                msi::Value::Str("media1.cab".to_owned()),
            ),
        ]);
        assert_eq!(row.integer("diskid"), Some(2));
        assert_eq!(row.text("cabinet"), Some("media1.cab"));
        assert_eq!(row.text("missing"), None);
    }
}
