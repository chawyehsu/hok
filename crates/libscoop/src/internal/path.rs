#![allow(dead_code)]
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Fallible};

/// Return the Leaf, i.e. file name (with extension), or directory name
/// of given path.
#[inline(always)]
pub fn leaf<P: AsRef<Path> + ?Sized>(path: &P) -> Option<&str> {
    path.as_ref().file_name().and_then(|s| s.to_str())
}

/// Return the LeafBase, i.e. file name without extension, for given file path.
///
/// If the given path is a directory, it returns the `Leaf` of the path instead.
#[inline(always)]
pub fn leaf_base<P: AsRef<Path> + ?Sized>(path: &P) -> Option<&str> {
    path.as_ref().file_stem().and_then(|s| s.to_str())
}

pub fn extract_name_and_bucket(path: &Path) -> Fallible<(String, String)> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        // FIXME: Uppercase <name> is not a good idea, the support is going to be dropped.
        let p = r".*?[\\/]buckets[\\/](?P<bucket>[a-zA-Z0-9-_]+).*?[\\/](?P<name>[a-zA-Z0-9-_@.]+).json$";
        RegexBuilder::new(p).build().unwrap()
    });
    match RE.captures(path.to_str().unwrap()) {
        None => {}
        Some(caps) => {
            let name = caps.name("name").map(|m| m.as_str().to_string());
            let bucket = caps.name("bucket").map(|m| m.as_str().to_string());
            if let Some(name) = name {
                if let Some(bucket) = bucket {
                    return Ok((name, bucket));
                }
            }
        }
    }

    Err(Error::Custom(format!(
        "unsupported manifest path {}",
        path.display()
    )))
}

/// Normalize a path, removing things like `.` and `..`.
///
/// CAUTION: This does not resolve symlinks (unlike
/// [`std::fs::canonicalize`]). This may cause incorrect or surprising
/// behavior at times. This should be used carefully. Unfortunately,
/// [`std::fs::canonicalize`] can be hard to use correctly, since it can often
/// fail, or on Windows returns annoying device paths.
///
/// This function is copied from Cargo.
pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut components = path.as_ref().components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}
