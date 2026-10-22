//! Atomic assembly and commit of package version directories.
//!
//! A package version is assembled in a private staging directory that is a
//! sibling of the live package version directory, so promoting it is a single
//! same-volume rename. A failed or interrupted assembly therefore never touches
//! the live directory; the swap is the only commit point.

use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use tracing::debug;
use tracing::warn;

use crate::error::Fallible;

/// Name of the directory holding all staging directories of a package.
const STAGING_DIR: &str = ".staging";

/// Name of the marker file written when assembly is complete.
pub const COMPLETE_MARKER: &str = ".complete";

/// Suffix appended to a version to name its retired (pre-swap) directory.
const RETIRED_SUFFIX: &str = ".old";

/// A package version directory under assembly.
///
/// The directory is created eagerly by [`staging_dir`]. Its layout is
/// `apps/<name>/.staging/<version>`, nested under the live package directory so
/// that the commit rename never crosses volumes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StagingDir {
    path: PathBuf,
}

/// Create a staging directory for a package version, e.g.
/// `apps/<name>/.staging/<version>`.
///
/// The parent package directory is created when missing. Files left over from a
/// previous attempt at the same version are cleared so the assembly always
/// starts from an empty directory.
pub fn staging_dir(apps_dir: &Path, name: &str, version: &str) -> Fallible<StagingDir> {
    let path = apps_dir.join(name).join(STAGING_DIR).join(version);
    prepare_empty_dir(&path)?;
    Ok(StagingDir { path })
}

impl StagingDir {
    /// Path to assemble the package version into.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write the completeness marker. Call only when assembly is finished.
    pub fn complete(&self) -> Fallible<()> {
        fs::write(self.path.join(COMPLETE_MARKER), b"")?;
        Ok(())
    }
}

/// Commit a staged package version to `apps/<name>/<version>`, replacing any
/// existing directory.
///
/// An existing live directory is renamed out of the way first, then the staging
/// directory is renamed into its place. When that second rename fails the
/// retired directory is restored, so a failure never loses the live version.
pub fn commit_swap(live: &Path, staging: &StagingDir) -> Fallible<()> {
    let live_exists = live.try_exists()?;
    let retired = match live_exists {
        true => Some(retire(live)?),
        false => None,
    };

    let staging_path = staging.path();
    if let Err(err) = fs::rename(staging_path, live) {
        if let Some(retired) = &retired
            && let Err(restore_err) = fs::rename(retired, live)
        {
            warn!(
                "failed to restore '{}' to '{}' after a failed commit (err: {})",
                retired.display(),
                live.display(),
                restore_err
            );
        }
        return Err(err.into());
    }

    if let Some(retired) = retired {
        remove_best_effort(&retired);
    }

    Ok(())
}

/// Remove stale staging / retired dirs for a package version. Best effort.
pub fn sweep(apps_dir: &Path, name: &str) {
    let staging_root = apps_dir.join(name).join(STAGING_DIR);
    let entries = match fs::read_dir(&staging_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return,
        Err(err) => {
            debug!(
                "failed to read staging directory '{}' (err: {})",
                staging_root.display(),
                err
            );
            return;
        }
    };

    for entry in entries.flatten() {
        remove_best_effort(&entry.path());
    }

    if let Err(err) = fs::remove_dir(&staging_root)
        && err.kind() != io::ErrorKind::NotFound
    {
        debug!(
            "failed to remove staging directory '{}' (err: {})",
            staging_root.display(),
            err
        );
    }
}

/// Rename the live directory to a unique retired name under `.staging`.
fn retire(live: &Path) -> Fallible<PathBuf> {
    let version = live
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "live path has no file name"))?;
    let staging_root = live
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "live path has no parent"))?
        .join(STAGING_DIR);
    fs::create_dir_all(&staging_root)?;

    let retired = unique_tombstone(&staging_root, version);
    fs::rename(live, &retired)?;
    Ok(retired)
}

/// Pick a path under `staging_root` that does not exist yet.
///
/// Tries `<version>.old`, then `<version>.old.<pid>.<n>` for increasing `n`.
fn unique_tombstone(staging_root: &Path, version: &OsStr) -> PathBuf {
    let preferred = staging_root.join(tombstone_name(version, None));
    if !preferred.exists() {
        return preferred;
    }

    let pid = std::process::id();
    (1..=u32::MAX)
        .map(|n| staging_root.join(tombstone_name(version, Some((pid, n)))))
        .find(|candidate| !candidate.exists())
        .unwrap_or(preferred)
}

fn tombstone_name(version: &OsStr, unique: Option<(u32, u32)>) -> OsString {
    let mut name = version.to_os_string();
    name.push(RETIRED_SUFFIX);
    if let Some((pid, n)) = unique {
        name.push(format!(".{pid}.{n}"));
    }
    name
}

/// Remove a stale directory or file, never reporting failure.
fn remove_best_effort(path: &Path) {
    let result = match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_dir() => crate::internal::fs::remove_dir(path),
        Ok(_) => fs::remove_file(path),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    };
    if let Err(err) = result
        && err.kind() != io::ErrorKind::NotFound
    {
        debug!(
            "failed to remove stale path '{}' (err: {})",
            path.display(),
            err
        );
    }
}

/// Create `path` as an empty directory, clearing any previous contents.
fn prepare_empty_dir(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_dir() => crate::internal::fs::empty_dir(path),
        Ok(_) => {
            fs::remove_file(path)?;
            fs::create_dir_all(path)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => fs::create_dir_all(path),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn apps() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write_file(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn read_file(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    fn staging_root(apps_dir: &Path, name: &str) -> PathBuf {
        apps_dir.join(name).join(STAGING_DIR)
    }

    #[test]
    fn staging_path_is_nested_under_package() {
        let apps = apps();
        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();

        assert_eq!(
            staging.path(),
            apps.path().join("foo").join(".staging").join("1.0.0")
        );
        assert!(staging.path().is_dir());
    }

    #[test]
    fn staging_dir_is_created_when_package_dir_is_missing() {
        let apps = apps();
        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();

        assert!(apps.path().join("foo").is_dir());
        assert!(staging.path().is_dir());
    }

    #[test]
    fn staging_dir_resets_previous_attempt() {
        let apps = apps();
        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();
        write_file(&staging.path().join("leftover.txt"), "stale");

        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();

        assert!(!staging.path().join("leftover.txt").exists());
        assert!(staging.path().is_dir());
    }

    #[test]
    fn complete_writes_marker() {
        let apps = apps();
        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();
        staging.complete().unwrap();

        assert!(staging.path().join(COMPLETE_MARKER).is_file());
    }

    #[test]
    fn commit_into_empty_app_dir() {
        let apps = apps();
        let live = apps.path().join("foo").join("1.0.0");
        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();
        write_file(&staging.path().join("bin").join("foo.exe"), "binary");
        staging.complete().unwrap();

        commit_swap(&live, &staging).unwrap();

        assert_eq!(read_file(&live.join("bin").join("foo.exe")), "binary");
        assert!(live.join(COMPLETE_MARKER).is_file());
        assert!(!staging.path().exists());
    }

    #[test]
    fn commit_replaces_existing_version() {
        let apps = apps();
        let live = apps.path().join("foo").join("1.0.0");
        write_file(&live.join("old.txt"), "old");

        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();
        write_file(&staging.path().join("new.txt"), "new");
        staging.complete().unwrap();

        commit_swap(&live, &staging).unwrap();

        assert_eq!(read_file(&live.join("new.txt")), "new");
        assert!(!live.join("old.txt").exists());
        assert!(!staging.path().exists());
    }

    #[test]
    fn commit_cleans_up_retired_directory() {
        let apps = apps();
        let live = apps.path().join("foo").join("1.0.0");
        write_file(&live.join("old.txt"), "old");

        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();
        staging.complete().unwrap();

        commit_swap(&live, &staging).unwrap();

        let leftovers = fs::read_dir(staging_root(apps.path(), "foo"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect::<Vec<_>>();
        assert!(leftovers.is_empty(), "leftovers: {leftovers:?}");
    }

    #[test]
    fn commit_restores_retired_directory_when_promotion_fails() {
        let apps = apps();
        let live = apps.path().join("foo").join("1.0.0");
        write_file(&live.join("old.txt"), "old");

        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();
        staging.complete().unwrap();
        crate::internal::fs::remove_dir(staging.path()).unwrap();

        let result = commit_swap(&live, &staging);

        assert!(result.is_err());
        assert_eq!(read_file(&live.join("old.txt")), "old");

        let retired = staging_root(apps.path(), "foo").join("1.0.0.old");
        assert!(!retired.exists(), "retired directory was not restored");
    }

    #[test]
    fn commit_restores_retired_directory_when_source_disappears() {
        let apps = apps();
        let live = apps.path().join("foo").join("1.0.0");
        write_file(&live.join("old.txt"), "old");

        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();
        let missing = staging.path().join("nested");
        let mut absent_staging = staging.clone();
        absent_staging.path = missing;

        let result = commit_swap(&live, &absent_staging);

        assert!(result.is_err());
        assert_eq!(read_file(&live.join("old.txt")), "old");
        assert!(!staging_root(apps.path(), "foo").join("1.0.0.old").exists());
    }

    #[test]
    fn tombstone_is_unique_when_retired_name_exists() {
        let apps = apps();
        let live = apps.path().join("foo").join("1.0.0");
        write_file(&live.join("old.txt"), "old");

        let stale = staging_root(apps.path(), "foo").join("1.0.0.old");
        write_file(&stale.join("stale.txt"), "stale");

        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();
        staging.complete().unwrap();

        commit_swap(&live, &staging).unwrap();

        assert_eq!(read_file(&live.join(COMPLETE_MARKER)), "");
        assert_eq!(read_file(&stale.join("stale.txt")), "stale");
    }

    #[test]
    fn sweep_removes_leftovers() {
        let apps = apps();
        let foo_staging = staging_root(apps.path(), "foo");
        write_file(&foo_staging.join("1.0.0").join("partial.txt"), "partial");
        write_file(&foo_staging.join("1.0.0.old").join("stale.txt"), "stale");
        write_file(
            &foo_staging.join("1.0.0.old.42.1").join("stale.txt"),
            "stale",
        );

        let other = staging_root(apps.path(), "bar");
        write_file(&other.join("2.0.0").join("keep.txt"), "keep");

        sweep(apps.path(), "foo");

        assert!(!foo_staging.exists());
        assert!(other.join("2.0.0").join("keep.txt").is_file());
    }

    #[test]
    fn sweep_is_noop_without_staging_directory() {
        let apps = apps();
        sweep(apps.path(), "missing");
    }

    #[test]
    fn staging_is_invisible_to_installed_package_queries() {
        // `query_installed` only considers an `apps/<name>` entry installed when
        // `apps/<name>/current` holds both a manifest and an install file. The
        // staging directory is nested below `apps/<name>/.staging`, so neither
        // it nor the package directory it creates can look installed.
        let apps = apps();
        let staging = staging_dir(apps.path(), "foo", "1.0.0").unwrap();
        write_file(&staging.path().join("scoop-manifest.json"), "{}");
        write_file(&staging.path().join("scoop-install.json"), "{}");

        let package_dir = apps.path().join("foo");
        let relative = staging.path().strip_prefix(&package_dir).unwrap();
        assert_eq!(
            relative.components().next().unwrap().as_os_str(),
            STAGING_DIR
        );
        assert!(!package_dir.join("current").exists());

        let entries = fs::read_dir(&package_dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or_default())
            .map(|e| e.file_name())
            .collect::<Vec<_>>();
        assert_eq!(entries, vec![OsString::from(STAGING_DIR)]);
    }
}
