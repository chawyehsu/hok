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

/// Decompression budget for the `nsis` crate.
///
/// The crate applies one budget to every stream of unknown size, including the
/// solid stream that holds the whole installed tree, and records
/// `SolidStatus::Truncated` past it; the truncated entries then fail with
/// `Error::OutputTooLarge`. The crate's own default is 64 MiB, which solid
/// installers of large packages exceed. The bound is a fixed ceiling rather
/// than a guess derived from the download, so a prepared installer cannot buy
/// itself a larger allocation. Installers whose data does not fit fall back to
/// the helper.
const DECOMPRESS_BUDGET: usize = 2 << 30;

/// [`DECOMPRESS_BUDGET`] in the largest binary unit that divides it, for
/// messages.
const DECOMPRESS_BUDGET_LABEL: &str = "2 GiB";

/// Extract an NSIS installer into `dest`.
pub fn extract(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let _ = probe;
    match nsis_extract(src, dest, observer) {
        Ok(summary) => Ok(summary),
        Err(reason) => {
            tracing::debug!(
                "falling back to 7z.exe for nsis installer: {}",
                reason.warning()
            );
            let mut summary = extract_helper(src, dest, observer)?;
            summary.warnings.push(reason.warning());
            Ok(summary)
        }
    }
}

/// Why the `nsis` crate path was not used.
#[derive(Debug)]
enum Fallback {
    /// The installer is too large for the crate's decompression budget, so the
    /// crate was not attempted.
    TooLarge,
    /// The crate ran and failed.
    Failed(Error),
}

impl Fallback {
    /// The warning recorded on the summary produced by the helper.
    fn warning(&self) -> String {
        match self {
            Fallback::TooLarge => format!(
                "installer exceeds the {DECOMPRESS_BUDGET_LABEL} decompression budget; used 7z.exe"
            ),
            Fallback::Failed(err) => format!("nsis crate failed ({err}); used 7z.exe"),
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
) -> Result<ExtractSummary, Fallback> {
    let data = std::fs::read(src).map_err(|err| Fallback::Failed(error(src, err)))?;
    if data.len() > DECOMPRESS_BUDGET {
        return Err(Fallback::TooLarge);
    }
    let installer = nsis::NsisInstaller::builder(&data)
        .max_decompressed_size(DECOMPRESS_BUDGET)
        .parse()
        .map_err(|err| Fallback::Failed(error(src, err)))?;
    write_entries(src, &installer, dest, observer).map_err(Fallback::Failed)
}

#[cfg(windows)]
fn write_entries(
    src: &Path,
    installer: &nsis::NsisInstaller<'_>,
    dest: &Path,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
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
        let Some(name) = install_root_relative(&path) else {
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
    if entries == 0 {
        return Err(error(
            src,
            format!(
                "the nsis crate reported no files in the install root: {}",
                warnings.join("; ")
            ),
        ));
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
) -> Result<ExtractSummary, Fallback> {
    Err(Fallback::Failed(Error::Extract {
        archive: PathBuf::new(),
        reason: "the nsis crate is only available on Windows".to_owned(),
    }))
}

/// The entry name of a file that lands in the install root, or `None`.
///
/// A `File` instruction stores its destination relative to `$INSTDIR`, so most
/// entries arrive as a plain relative path (`code/foo.dll`). Some installers
/// spell the root out (`$INSTDIR/foo.dll`). Anything targeting another
/// variable (`$PLUGINSDIR`, `$SYSDIR`, ...) or an absolute path is not package
/// content, and plugin staging is dropped to match the helper path.
fn install_root_relative(name: &str) -> Option<String> {
    let name = name.replace('\\', "/");
    let rest = name
        .strip_prefix("$INSTDIR/")
        .or_else(|| name.strip_prefix("$instdir/"))
        .unwrap_or(&name);
    if rest.is_empty() || rest.starts_with('/') || rest.contains('$') {
        return None;
    }
    let relative = Path::new(rest);
    if relative.is_absolute() {
        return None;
    }
    let plugin_staging = relative
        .components()
        .next()
        .and_then(|first| match first {
            std::path::Component::Normal(part) => Some(part.eq_ignore_ascii_case("_plugins")),
            _ => None,
        })
        .unwrap_or(false);
    if plugin_staging {
        return None;
    }
    Some(rest.to_owned())
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
    fn install_root_relative_strips_prefix() {
        assert_eq!(
            install_root_relative("$INSTDIR/tools/a.exe").as_deref(),
            Some("tools/a.exe")
        );
        assert_eq!(
            install_root_relative("$instdir/a.exe").as_deref(),
            Some("a.exe")
        );
        assert_eq!(
            install_root_relative("$INSTDIR\\a.exe").as_deref(),
            Some("a.exe")
        );
        assert_eq!(install_root_relative("$PLUGINSDIR/x.dll"), None);
        assert_eq!(install_root_relative("$INSTDIR"), None);
    }

    #[test]
    fn install_root_relative_accepts_plain_relative_paths() {
        assert_eq!(
            install_root_relative("code/package.json").as_deref(),
            Some("code/package.json")
        );
        assert_eq!(
            install_root_relative("wechatdevtools.exe").as_deref(),
            Some("wechatdevtools.exe")
        );
        assert_eq!(
            install_root_relative("bin\\tool.dll").as_deref(),
            Some("bin/tool.dll")
        );
    }

    #[test]
    fn install_root_relative_rejects_non_payload_paths() {
        assert_eq!(install_root_relative(""), None);
        assert_eq!(install_root_relative("/absolute/a.exe"), None);
        assert_eq!(install_root_relative("$SYSDIR/kernel32.dll"), None);
        assert_eq!(install_root_relative("$TEMP\\a.tmp"), None);
        assert_eq!(install_root_relative("_plugins/System.dll"), None);
        assert_eq!(install_root_relative("_PLUGINS/nsDialogs.dll"), None);
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
    fn oversized_installer_is_reported_as_too_large() {
        let warning = Fallback::TooLarge.warning();
        assert!(
            warning.contains(DECOMPRESS_BUDGET_LABEL),
            "warning: {warning}"
        );
        assert!(warning.contains("7z.exe"), "warning: {warning}");
    }

    #[test]
    fn crate_failure_names_the_crate_and_the_helper() {
        let error = Error::Extract {
            archive: PathBuf::from("a.exe"),
            reason: "bad signature".to_owned(),
        };
        let warning = Fallback::Failed(error).warning();
        assert!(warning.contains("bad signature"), "warning: {warning}");
        assert!(warning.contains("7z.exe"), "warning: {warning}");
    }

    #[test]
    fn budget_is_a_fixed_bound_above_the_crate_default() {
        const {
            assert!(DECOMPRESS_BUDGET > nsis::NsisInstaller::DEFAULT_MAX_DECOMPRESSED_SIZE);
            assert!(DECOMPRESS_BUDGET == 2 << 30);
        }
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
        let name = install_root_relative("$INSTDIR/../../evil.exe").unwrap();
        assert!(super::super::guard(&dest, Path::new(&name)).is_err());
    }
}
