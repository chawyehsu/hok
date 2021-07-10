use once_cell::sync::Lazy;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use regex::Regex;
use regex::RegexBuilder;
use std::io;
use std::path::Path;
use std::path::PathBuf;

/// Ensure given `path` exist.
///
/// Will call [`std::fs::create_dir_all`] if `path` doesn't exist.
pub fn ensure_dir<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<()> {
    match path.as_ref().exists() {
        false => std::fs::create_dir_all(path.as_ref()),
        true => Ok(()),
    }
}

#[inline(always)]
pub fn empty_dir<P: AsRef<Path> + ?Sized>(path: &P) -> io::Result<()> {
    match path.as_ref().exists() {
        true => remove_dir_all::remove_dir_contents(path.as_ref()),
        false => Ok(()),
    }
}

/// Read all JSON files in the given `path` (parallelly) and return a list of
/// [`PathBuf`]s of these JSON files.
///
/// Note: this function will ignore JSON files named `package.json`, which is
/// considered to be the config file a NPM package.
pub fn walk_dir_json(path: &Path) -> io::Result<Vec<PathBuf>> {
    Ok(path
        .read_dir()?
        .filter_map(io::Result::ok)
        .par_bridge()
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

/// Convert bytes to KB/MB/GB representation.
pub fn filesize(length: u64, with_unit: bool) -> String {
    let gb: f64 = 2.0_f64.powf(30_f64);
    let mb: f64 = 2.0_f64.powf(20_f64);
    let kb: f64 = 2.0_f64.powf(10_f64);

    let flength = length as f64;

    if flength > gb {
        let j = (flength / gb).round();

        if with_unit {
            format!("{} GB", j)
        } else {
            j.to_string()
        }
    } else if flength > mb {
        let j = (flength / mb).round();

        if with_unit {
            format!("{} MB", j)
        } else {
            j.to_string()
        }
    } else if flength > kb {
        let j = (flength / kb).round();

        if with_unit {
            format!("{} KB", j)
        } else {
            j.to_string()
        }
    } else {
        if with_unit {
            format!("{} B", flength)
        } else {
            flength.to_string()
        }
    }
}
