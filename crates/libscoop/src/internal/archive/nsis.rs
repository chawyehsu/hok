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

/// Name of the helper extractor, as reported in an extraction summary.
const HELPER_EXTRACTOR: &str = "nsis (helper: 7z.exe)";

/// Entries a 7z extraction of an NSIS installer produces that are not part of
/// the installed package: the installer's own temporary directory and its
/// uninstaller.
const INSTALLER_ARTIFACTS: &[&str] = &["$PLUGINSDIR", "Uninstall.exe"];

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
            extract_helper(src, dest, observer)
        }
    }
}

/// Extract through `7z.exe`, then drop the entries the installer would have
/// created only at install time.
fn extract_helper(
    src: &Path,
    dest: &Path,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    helper::require(helper::SEVEN_ZIP)?;
    let mut args = SEVEN_ZIP_ARGS
        .iter()
        .map(|flag| (*flag).to_owned())
        .collect::<Vec<_>>();
    args.push(format!("-o{}", dest.display()));
    args.push(src.display().to_string());
    helper::run(helper::SEVEN_ZIP, args, None)?;
    remove_installer_artifacts(dest);
    helper::report_tree(src, dest, observer, Vec::new(), HELPER_EXTRACTOR)
}

/// Remove the installer artifacts from an extracted tree.
///
/// Failures are ignored: the entries are not part of the package, so a tree
/// that keeps them is still usable.
fn remove_installer_artifacts(dest: &Path) {
    for name in INSTALLER_ARTIFACTS {
        let path = dest.join(name);
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else {
            let _ = std::fs::remove_file(&path);
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
    fn installer_artifacts_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path();
        std::fs::create_dir_all(dest.join("$PLUGINSDIR")).unwrap();
        std::fs::write(dest.join("$PLUGINSDIR/Plugin.dll"), b"p").unwrap();
        std::fs::write(dest.join("Uninstall.exe"), b"u").unwrap();
        std::fs::write(dest.join("app.exe"), b"a").unwrap();
        remove_installer_artifacts(dest);
        assert!(!dest.join("$PLUGINSDIR").exists());
        assert!(!dest.join("Uninstall.exe").exists());
        assert!(dest.join("app.exe").is_file());
    }

    #[test]
    fn missing_artifacts_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.exe"), b"a").unwrap();
        remove_installer_artifacts(dir.path());
        assert!(dir.path().join("app.exe").is_file());
    }

    #[test]
    fn instdir_guard_rejects_escapes() {
        let dest = std::env::temp_dir();
        let name = instdir_relative("$INSTDIR/../../evil.exe").unwrap();
        assert!(super::super::guard(&dest, Path::new(&name)).is_err());
    }
}
