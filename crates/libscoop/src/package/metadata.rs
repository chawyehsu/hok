//! Installed package metadata file resolution.
//!
//! Scoop prefixes the installed package metadata files with `scoop-` to
//! avoid clobbering files having the same name shipped by the app itself
//! since Scoop 0.6.0 [ScoopInstaller/Scoop#6732][1].
//!
//! The legacy names (`manifest.json` and `install.json`) are still read by
//! Scoop for backward compatibility, so both layouts must be supported here,
//! the prefixed names are preferred when both exist.
//!
//! [1]: https://github.com/ScoopInstaller/Scoop/pull/6732

use std::path::{Path, PathBuf};

/// The app manifest file name used by Scoop 0.6.0 and later.
const MANIFEST: &str = "scoop-manifest.json";

/// The install manifest file name used by Scoop 0.6.0 and later.
const INSTALL: &str = "scoop-install.json";

/// The app manifest file name used before Scoop 0.6.0.
const LEGACY_MANIFEST: &str = "manifest.json";

/// The install manifest file name used before Scoop 0.6.0.
const LEGACY_INSTALL: &str = "install.json";

fn resolve(dir: &Path, new: &str, old: &str) -> PathBuf {
    let new_path = dir.join(new);
    if matches!(new_path.try_exists(), Ok(false)) {
        dir.join(old)
    } else {
        new_path
    }
}

/// Resolve the manifest json path of an installed app's version directory.
pub fn resolve_manifest_json_path(version_dir: impl AsRef<Path>) -> PathBuf {
    resolve(version_dir.as_ref(), MANIFEST, LEGACY_MANIFEST)
}

/// Resolve the install json path of an installed app's version directory.
pub fn resolve_install_json_path(version_dir: impl AsRef<Path>) -> PathBuf {
    resolve(version_dir.as_ref(), INSTALL, LEGACY_INSTALL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{OsStr, OsString};

    fn touch(dir: &Path, names: &[&str]) {
        for name in names {
            std::fs::write(dir.join(name), "{}").unwrap();
        }
    }

    fn file_name(path: PathBuf) -> OsString {
        path.file_name().unwrap().to_owned()
    }

    #[test]
    fn prefers_prefixed_names() {
        let dir = tempfile::tempdir().unwrap();
        touch(
            dir.path(),
            &[LEGACY_MANIFEST, LEGACY_INSTALL, MANIFEST, INSTALL],
        );

        assert_eq!(
            file_name(resolve_manifest_json_path(dir.path())),
            OsStr::new(MANIFEST)
        );
        assert_eq!(
            file_name(resolve_install_json_path(dir.path())),
            OsStr::new(INSTALL)
        );
    }

    #[test]
    fn falls_back_to_legacy_names() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &[LEGACY_MANIFEST, LEGACY_INSTALL]);

        assert_eq!(
            file_name(resolve_manifest_json_path(dir.path())),
            OsStr::new(LEGACY_MANIFEST)
        );
        assert_eq!(
            file_name(resolve_install_json_path(dir.path())),
            OsStr::new(LEGACY_INSTALL)
        );
    }

    #[test]
    fn falls_back_when_nothing_exists() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            file_name(resolve_manifest_json_path(dir.path())),
            OsStr::new(LEGACY_MANIFEST)
        );
        assert_eq!(
            file_name(resolve_install_json_path(dir.path())),
            OsStr::new(LEGACY_INSTALL)
        );
    }
}
