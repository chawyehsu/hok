//! Nsis extractor.
//!
//! The `nsis` crate is the primary path on Windows. Wherever it is unavailable,
//! or when it cannot parse the installer, the `7z.exe` helper runs.

use std::path::Path;
use std::path::PathBuf;

use super::ExtractObserver;
use super::ExtractSummary;
use super::Probe;
use super::helper;
use super::helper::SEVEN_ZIP_ARGS;
use crate::error::Error;
use crate::error::Fallible;

/// The `$INSTDIR` prefix NSIS uses for files that land in the install root.
const INSTDIR: &str = "$INSTDIR";

/// Extract an NSIS installer into `dest`.
pub fn extract(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    match nsis_extract(src, dest, observer) {
        Ok(summary) => Ok(summary),
        Err(err) => {
            tracing::debug!(
                "failed to extract nsis installer with nsis crate (err: {})",
                err
            );
            let _ = probe;
            helper::extract_archive(helper::SEVEN_ZIP, SEVEN_ZIP_ARGS, src, dest, observer)
        }
    }
}

/// Extract an NSIS installer, falling back to the `7z.exe` helper.
pub fn extract_with_fallback(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    helper::require(helper::SEVEN_ZIP)?;
    extract(src, dest, probe, observer)
}

#[cfg(windows)]
fn nsis_extract(
    src: &Path,
    dest: &Path,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let data = std::fs::read(src).map_err(|err| error(src, err))?;
    let installer = nsis::NsisInstaller::from_bytes(&data).map_err(|err| error(src, err))?;
    let mut entries = 0u64;
    let mut bytes = 0u64;
    let mut warnings = Vec::new();
    for file in installer.files() {
        let file = match file {
            Ok(file) => file,
            Err(err) => {
                warnings.push(format!("skipped entry: {err}"));
                continue;
            }
        };
        let path = match file.dest_path() {
            Ok(path) => path.to_path(),
            Err(err) => {
                warnings.push(format!("skipped entry: {err}"));
                continue;
            }
        };
        let Some(name) = instdir_relative(&path) else {
            continue;
        };
        let target = super::guard(dest, Path::new(&name))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        observer.entry_written(&name);
        let payload = match file.decompress() {
            Ok(payload) => payload,
            Err(err) => {
                warnings.push(format!("skipped entry '{name}': {err}"));
                continue;
            }
        };
        std::fs::write(&target, &payload)?;
        entries = entries.saturating_add(1);
        bytes = bytes.saturating_add(payload.len() as u64);
    }
    Ok(ExtractSummary {
        extractor: "nsis",
        entries,
        bytes,
        warnings,
    })
}

#[cfg(not(windows))]
fn nsis_extract(
    _src: &Path,
    _dest: &Path,
    _observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    Err(Error::Extract {
        archive: PathBuf::new(),
        reason: "the nsis crate is only available on Windows".to_owned(),
    })
}

/// Strip `$INSTDIR/` (or the lowercase form) from an entry name.
fn instdir_relative(name: &str) -> Option<String> {
    let name = name.replace('\\', "/");
    let rest = name
        .strip_prefix("$INSTDIR/")
        .or_else(|| name.strip_prefix("$instdir/"))?;
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
    fn instdir_relative_strips_prefix() {
        assert_eq!(
            instdir_relative("$INSTDIR/tools/a.exe").as_deref(),
            Some("tools/a.exe")
        );
        assert_eq!(instdir_relative("$instdir/a.exe").as_deref(), Some("a.exe"));
        assert_eq!(
            instdir_relative("$INSTDIR\\a.exe").as_deref(),
            Some("a.exe")
        );
        assert_eq!(instdir_relative("$PLUGINSDIR/x.dll"), None);
        assert_eq!(instdir_relative("$INSTDIR"), None);
    }

    #[test]
    fn instdir_guard_rejects_escapes() {
        let dest = std::env::temp_dir();
        let name = instdir_relative("$INSTDIR/../../evil.exe").unwrap();
        assert!(super::super::guard(&dest, Path::new(&name)).is_err());
    }
}
