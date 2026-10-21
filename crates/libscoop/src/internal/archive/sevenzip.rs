//! 7z extractor backed by `sevenz-rust2`.

use std::fs;
use std::fs::File;
use std::fs::FileTimes;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use super::ExtractObserver;
use super::ExtractSummary;
use super::Probe;
use super::helper;
use super::helper::Helper;
use super::helper::SEVEN_ZIP_ARGS;
use crate::error::Error;
use crate::error::Fallible;

/// Extract a 7z archive into `dest`, seeking to [`Probe::offset`] first.
pub fn extract(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let mut file = fs::File::open(src)?;
    super::zip::skip_to(&mut file, probe.offset)?;

    let mut reader = sevenz_rust2::ArchiveReader::new(file, sevenz_rust2::Password::empty())
        .map_err(|err| archive_error(src, err))?;

    let mut out = Outcome::default();
    let mut passthrough = None;
    let result =
        reader.for_each_entries(
            |entry, input| match out.visit(dest, observer, entry, input) {
                Ok(keep_going) => Ok(keep_going),
                Err(err) => {
                    passthrough = Some(err);
                    Err(io::Error::other("entry rejected").into())
                }
            },
        );

    match (result, passthrough) {
        (_, Some(err)) => Err(err),
        (Ok(()), None) => {
            out.apply_times();
            Ok(ExtractSummary {
                extractor: "7z",
                entries: out.entries,
                bytes: out.bytes,
                warnings: out.warnings,
            })
        }
        (Err(err), None) => Err(archive_error(src, err)),
    }
}

/// Extract a 7z archive, falling back to the `7z.exe` helper.
pub fn extract_with_fallback(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    match extract(src, dest, probe, observer) {
        Ok(summary) => Ok(summary),
        Err(err) if !Helper::SevenZip.available() => Err(err),
        Err(_) => helper::extract_archive(
            Helper::SevenZip,
            helper::SEVEN_ZIP_ARGS,
            src,
            dest,
            observer,
        ),
    }
}

fn archive_error(archive: &Path, err: sevenz_rust2::Error) -> Error {
    Error::Extract {
        archive: archive.to_path_buf(),
        reason: err.to_string(),
    }
}

#[derive(Default)]
struct Outcome {
    entries: u64,
    bytes: u64,
    warnings: Vec<String>,
    times: Vec<(PathBuf, std::time::SystemTime)>,
}

impl Outcome {
    fn visit(
        &mut self,
        dest: &Path,
        observer: &dyn ExtractObserver,
        entry: &sevenz_rust2::ArchiveEntry,
        input: &mut dyn Read,
    ) -> Fallible<bool> {
        let name = entry.name().replace('\\', "/");
        let target = super::guard(dest, Path::new(&name))?;

        if entry.is_directory() {
            fs::create_dir_all(&target)?;
            return Ok(true);
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        observer.entry_written(&name);
        let written = match write_file(input, &target) {
            Ok(written) => written,
            Err(err) => {
                self.warnings.push(format!("skipped entry '{name}': {err}"));
                let _ = std::fs::remove_file(&target);
                return Ok(true);
            }
        };
        self.entries = self.entries.saturating_add(1);
        self.bytes = self.bytes.saturating_add(written);

        let modified = entry.last_modified_date();
        if u64::from(modified) != 0 {
            self.times.push((target, modified.into()));
        }
        Ok(true)
    }

    fn apply_times(&self) {
        for (path, time) in &self.times {
            if let Ok(file) = File::open(path) {
                let _ = file.set_times(FileTimes::new().set_modified(*time));
            }
        }
    }
}

fn write_file(input: &mut dyn Read, target: &Path) -> io::Result<u64> {
    let mut out = io::BufWriter::new(fs::File::create(target)?);
    let written = io::copy(input, &mut out)?;
    out.flush()?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_file_copies_all_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested").join("a.txt");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let written = write_file(&mut &b"abcdef"[..], &target).unwrap();
        assert_eq!(written, 6);
        assert_eq!(std::fs::read(&target).unwrap(), b"abcdef");
    }

    #[test]
    fn visit_rejects_parent_dir_entries() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let mut entry = sevenz_rust2::ArchiveEntry::new_file("../evil.txt");
        entry.has_stream = true;
        let mut outcome = Outcome::default();
        let observer = crate::internal::archive::NoopObserver;
        assert!(
            outcome
                .visit(&dest, &observer, &entry, &mut &b"x"[..])
                .is_err()
        );
    }

    #[test]
    fn visit_creates_directories_without_reporting_them() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        let observer = crate::internal::archive::NoopObserver;
        let mut outcome = Outcome::default();
        let entry = sevenz_rust2::ArchiveEntry::new_directory("nested/dir");
        assert!(
            outcome
                .visit(&dest, &observer, &entry, &mut &b""[..])
                .unwrap()
        );
        assert!(dest.join("nested/dir").is_dir());
        assert_eq!(outcome.entries, 0);
    }
}
