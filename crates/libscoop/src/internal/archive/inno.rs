//! Inno Setup extractor.
//!
//! `innospect` is the primary path on Windows. Wherever it is unavailable, or
//! when it cannot parse the installer, the `innounp.exe` helper runs. The core
//! applies `extract_dir` itself, so this extractor writes the whole `{app}`
//! subtree, or everything that is not a `{tmp}`/`{sys}` entry, and never writes
//! outside `dest`.

use std::io;
use std::path::Path;
use std::path::PathBuf;

use super::ExtractObserver;
use super::ExtractSummary;
use super::Probe;
use super::helper;
use super::helper::Helper;
use crate::error::Error;
use crate::error::Fallible;

/// The `{app}` subtree is what Scoop extracts: `innounp -c{app}`.
const APP: &str = "{app}";

/// Extract an Inno Setup installer into `dest`.
pub fn extract(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let result = crate::internal::archive::inno::innospect_extract(src, dest, probe, observer);
    match result {
        Ok(summary) => Ok(summary),
        Err(err) => {
            tracing::debug!(
                "failed to extract inno installer with innospect (err: {})",
                err
            );
            extract_helper(src, dest, observer)
        }
    }
}

/// Extract an Inno Setup installer, falling back to the `innounp.exe` helper.
pub fn extract_with_fallback(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    helper::require(Helper::Innounp)?;
    extract(src, dest, probe, observer)
}

/// The `innounp` arguments for extracting the `{app}` subtree into `dest`.
pub fn helper_args(src: &Path, dest: &Path) -> Vec<String> {
    vec![
        "-x".to_owned(),
        "-y".to_owned(),
        format!("-d{}", dest.display()),
        format!("-c{APP}"),
        src.display().to_string(),
    ]
}

fn extract_helper(
    src: &Path,
    dest: &Path,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    helper::require(Helper::Innounp)?;
    helper::run(Helper::Innounp, helper_args(src, dest), None)?;
    helper::report_tree(src, dest, observer, Vec::new(), Helper::Innounp.extractor())
}

/// The `innospect` extraction path. Only meaningful on Windows; other targets
/// report that the crate is unavailable so the helper is tried.
#[cfg(windows)]
fn innospect_extract(
    src: &Path,
    dest: &Path,
    _probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let data = std::fs::read(src).map_err(|err| error(src, err))?;
    let installer = innospect::InnoInstaller::from_bytes(&data).map_err(|err| error(src, err))?;
    let mut entries = 0u64;
    let mut bytes = 0u64;
    let mut warnings = Vec::new();
    for file in installer.files() {
        let Some(name) = app_relative(&file.destination) else {
            continue;
        };
        let target = super::guard(dest, Path::new(&name))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        observer.entry_written(&name);
        match installer.extract(file) {
            Ok(mut reader) => {
                let mut out = std::io::BufWriter::new(std::fs::File::create(&target)?);
                match std::io::copy(&mut reader, &mut out) {
                    Ok(written) => {
                        entries = entries.saturating_add(1);
                        bytes = bytes.saturating_add(written);
                    }
                    Err(err) => {
                        warnings.push(format!("skipped entry '{name}': {err}"));
                        let _ = std::fs::remove_file(&target);
                    }
                }
            }
            Err(err) => warnings.push(format!("skipped entry '{name}': {err}")),
        }
    }
    Ok(ExtractSummary {
        extractor: "innospect",
        entries,
        bytes,
        warnings,
    })
}

#[cfg(not(windows))]
fn innospect_extract(
    _src: &Path,
    _dest: &Path,
    _probe: &Probe,
    _observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    Err(Error::Extract {
        archive: PathBuf::new(),
        reason: "innospect is only available on Windows".to_owned(),
    })
}

/// Strip the `{app}/` prefix from an entry name, returning `None` for entries
/// outside the application directory.
fn app_relative(name: &str) -> Option<String> {
    let name = name.replace('\\', "/");
    let rest = name.strip_prefix(APP)?.strip_prefix('/')?;
    (!rest.is_empty()).then(|| rest.to_owned())
}

fn error(archive: &Path, err: impl std::fmt::Display) -> Error {
    Error::Extract {
        archive: archive.to_path_buf(),
        reason: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_args_select_the_app_subtree() {
        let args = helper_args(Path::new("setup.exe"), Path::new(r"C:\out"));
        assert_eq!(args[0], "-x");
        assert_eq!(args[1], "-y");
        assert_eq!(args[2], r"-dC:\out");
        assert_eq!(args[3], "-c{app}");
        assert_eq!(args[4], "setup.exe");
    }

    #[test]
    fn app_relative_strips_prefix() {
        assert_eq!(
            app_relative("{app}/bin/tool.exe").as_deref(),
            Some("bin/tool.exe")
        );
        assert_eq!(
            app_relative("{app}\\bin\\tool.exe").as_deref(),
            Some("bin/tool.exe")
        );
        assert_eq!(app_relative("{tmp}/x"), None);
        assert_eq!(app_relative("{app}"), None);
        assert_eq!(app_relative("{app}/"), None);
    }
}
