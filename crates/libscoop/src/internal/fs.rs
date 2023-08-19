use once_cell::sync::Lazy;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use regex::Regex;
use regex::RegexBuilder;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::error::Fallible;

/// Ensure given `path` exist.
#[inline]
pub fn ensure_dir<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<()> {
    std::fs::create_dir_all(path.as_ref())
}

/// Remove given `path` recursively.
#[inline]
pub fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    remove_dir_all::remove_dir_all(path)
}

/// Remove all files and subdirectories in given `path`.
///
/// This function will not remove the given `path` itself. No-op if the given
/// `path` does not exist.
#[inline(always)]
pub fn empty_dir<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<()> {
    let path = path.as_ref();
    match path.exists() {
        true => remove_dir_all::remove_dir_contents(path),
        false => Ok(()),
    }
}

/// Read all JSON files in the given `path` (parallelly) and return a list of
/// [`PathBuf`]s of these JSON files.
///
/// Note: this function will ignore JSON files named `package.json`, which is
/// considered to be the config file a NPM package.
pub fn walk_dir_json<P: AsRef<Path>>(path: P) -> io::Result<Vec<PathBuf>> {
    let path = path.as_ref();
    Ok(path
        .read_dir()?
        .par_bridge()
        .filter_map(io::Result::ok)
        .filter(|de| {
            let path = de.path();
            let name = path.file_name().unwrap().to_str().unwrap();
            // Only files, and avoid npm package config file
            path.is_file() && name.ends_with(".json") && name != "package.json"
        })
        .map(|de| de.path())
        .collect::<Vec<_>>())
}

/// Convert a string to a valid safe filename.
#[inline]
pub fn filenamify<S: AsRef<str>>(filename: S) -> String {
    static REGEX_REPLACE: Lazy<Regex> =
        Lazy::new(|| RegexBuilder::new(r"[^\w.-]+").build().unwrap());
    REGEX_REPLACE
        .replace_all(filename.as_ref(), "_")
        .into_owned()
}

/// Write given serializable data to a JSON file at given path.
///
/// This function will create the file if it does not exist, and truncate it.
pub fn write_json<P, D>(path: P, data: D) -> Fallible<()>
where
    P: AsRef<Path>,
    D: Serialize,
{
    let path = path.as_ref();
    ensure_dir(path.parent().unwrap())?;

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    Ok(serde_json::to_writer_pretty(file, &data)?)
}

/// Remove a symlink at `lnk`.
#[cfg(windows)]
pub fn remove_symlink<P: AsRef<Path>>(lnk: P) -> io::Result<()> {
    let lnk = lnk.as_ref();
    let metadata = lnk.symlink_metadata()?;
    let mut permissions = metadata.permissions();

    // Remove possible readonly flag on the symlink added by `attrib +R` command
    if permissions.readonly() {
        // Remove readonly flag
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        std::fs::set_permissions(lnk, permissions)?;
    }

    // We knew that `lnk` is a symlink but we don't know if it is a file or a
    // directory. So we need to check its metadata to determine how to remove
    // it. The file type of the symlink itself is always `FileType::Symlink`
    // and `symlink_metadata::is_dir` always returns `false` for symlinks, so
    // we have to check the metadata of the target file.
    if let Ok(target_metadata) = lnk.metadata() {
        if target_metadata.file_type().is_dir() {
            std::fs::remove_dir(lnk)
        } else {
            std::fs::remove_file(lnk)
        }
    } else {
        // We just can't get the metadata of the target file. It is possible
        // that the target file doesn't exist (perhaps it has been deleted).
        // The last thing we can do here is to use a mindless way to remove
        // the symlink.
        std::fs::remove_file(lnk).or_else(|_| std::fs::remove_dir(lnk))
    }
}

/// Remove a symlink at `lnk`.
#[cfg(unix)]
#[inline]
pub fn remove_symlink<P: AsRef<Path>>(lnk: P) -> io::Result<()> {
    std::fs::remove_file(lnk)
}

/// Create a directory symlink at `lnk` pointing to `src`.
#[cfg(windows)]
pub fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, lnk: Q) -> io::Result<()> {
    let src = src.as_ref();
    let lnk = lnk.as_ref();
    // Try to remove existing symlink, if any
    remove_symlink(lnk)?;

    // Ensure parent directory of `lnk` exists
    ensure_dir(lnk.parent().unwrap())?;
    // It is possible to create a symlink on Windows, but one of the following
    // conditions must be met:
    //
    // Either: the process has the `SeCreateSymbolicLinkPrivilege` privilege,
    // or: the OS is Windows 10 Creators Update or later and Developer Mode
    // enabled.
    //
    // We prefer symlink over junction because:
    // https://stackoverflow.com/questions/9042542/what-is-the-difference-between-ntfs-junction-points-and-symbolic-links
    //
    // Here we try to create a symlink first, and if it fails, we try to create
    // a junction which does not require any special privilege and works on
    // older versions of Windows.
    if std::os::windows::fs::symlink_dir(src, lnk).is_err() {
        junction::create(src, lnk)
    } else {
        Ok(())
    }
}

/// Create a directory symlink at `lnk` pointing to `src`.
#[cfg(unix)]
#[inline]
pub fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, lnk: Q) -> io::Result<()> {
    std::os::unix::fs::symlink(src, lnk)
}

/// Create a file symlink at `lnk` pointing to `src`.
#[cfg(windows)]
pub fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, lnk: Q) -> io::Result<()> {
    let src = src.as_ref();
    let lnk = lnk.as_ref();
    // Try to remove existing symlink, if any
    remove_symlink(lnk)?;

    // Ensure parent directory of `lnk` exists
    ensure_dir(lnk.parent().unwrap())?;
    // It is possible to create a symlink on Windows, but one of the following
    // conditions must be met:
    //
    // Either: the process has the `SeCreateSymbolicLinkPrivilege` privilege,
    // or: the OS is Windows 10 Creators Update or later and Developer Mode
    // enabled.
    //
    // We prefer symlink hence we try to create a symlink first, and if it fails,
    // a hard link will be created as a fallback.
    if std::os::windows::fs::symlink_file(src, lnk).is_err() {
        // It might not be a good idea to use hard link as a fallback for symlink,
        // which is absolutely not the same thing, but it is the best we can do
        // here and it seems to be suitable for our use case. Note that there
        // are *limitations* of hard links:
        // https://stackoverflow.com/questions/9042542/
        std::fs::hard_link(src, lnk)
    } else {
        Ok(())
    }
}

/// Create a file symlink at `lnk` pointing to `src`.
#[cfg(unix)]
#[inline]
pub fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, lnk: Q) -> io::Result<()> {
    std::os::unix::fs::symlink(src, lnk)
}

/// Create a symlink at `lnk` pointing to `src`.
/// This function will automatically determine if `src` is a file or a directory.
#[cfg(windows)]
pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, lnk: Q) -> io::Result<()> {
    let src = src.as_ref();
    let lnk = lnk.as_ref();

    let metadata = src.metadata()?;

    if metadata.file_type().is_dir() {
        symlink_dir(src, lnk)
    } else {
        symlink_file(src, lnk)
    }
}

/// Create a symlink at `lnk` pointing to `src`.
/// This function will automatically determine if `src` is a file or a directory.
#[cfg(unix)]
#[inline]
pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, lnk: Q) -> io::Result<()> {
    std::os::unix::fs::symlink(src, lnk)
}
