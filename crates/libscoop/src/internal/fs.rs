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
use crate::Error;

/// Ensure given `path` exist.
pub fn ensure_dir<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<()> {
    std::fs::create_dir_all(path.as_ref())
}

pub fn remove_dir<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<()> {
    remove_dir_all::remove_dir_all(path.as_ref())
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

/// Return the Leaf, i.e. file name (with extension), or directory name
/// of given path.
#[inline(always)]
pub fn leaf<P: AsRef<Path> + ?Sized>(path: &P) -> String {
    path.as_ref()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

/// Return the LeafBase, i.e. file name without extension, for given file path.
///
/// If the given path is a directory, it returns the [Leaf] of the path instead.
///
/// [Leaf]: self::leaf
#[inline]
pub fn leaf_base<P: AsRef<Path> + ?Sized>(path: &P) -> String {
    if path.as_ref().is_file() {
        path.as_ref()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    } else {
        self::leaf(path.as_ref())
    }
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
