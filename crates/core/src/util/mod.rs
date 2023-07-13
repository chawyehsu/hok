pub mod archive;
pub mod dag;
mod fs;
pub mod git;
mod tokio_util;

pub use fs::*;
use once_cell::sync::Lazy;
use regex::Regex;
use regex::RegexBuilder;
use std::path::Path;
pub use tokio_util::*;

use crate::error::{Error, Fallible};

pub fn os_is_arch64() -> bool {
    match std::mem::size_of::<&char>() {
        4 => false,
        8 => true,
        _ => panic!("unexpected os arch"),
    }
}

/// Check if a given executable is available on the system.
pub fn is_program_available(exe: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for p in path.split(";") {
            let path = Path::new(p).join(exe);
            if std::fs::metadata(path).is_ok() {
                return true;
            }
        }
    }
    false
}

/// FIXME: there is a wide knowledge of version comparsion,
/// And of cause I can't implement all of them at one commit, so plese fix me.
/// Perhaps https://github.com/timvisee/version-compare/issues/20 would
/// be a good reference.
pub fn compare_versions<S: AsRef<str>>(ver_a: S, ver_b: S) -> std::cmp::Ordering {
    let ver_a = ver_a.as_ref();
    let ver_b = ver_b.as_ref();
    let mut ver_a_parsed = ver_a.split(&['.', '-'][..]);
    let mut ver_b_parsed = ver_b.split(&['.', '-'][..]);

    loop {
        match ver_a_parsed.next() {
            Some(a_part) => {
                match ver_b_parsed.next() {
                    Some(b_part) => {
                        if a_part.parse::<u32>().is_ok() {
                            if b_part.parse::<u32>().is_ok() {
                                let a_part_num = a_part.parse::<u32>().unwrap();
                                let b_part_num = b_part.parse::<u32>().unwrap();

                                match a_part_num {
                                    n if n < b_part_num => return std::cmp::Ordering::Less,
                                    n if n > b_part_num => return std::cmp::Ordering::Greater,
                                    _ => continue,
                                }
                            }

                            // I guess this should be ok for cases like: 1.2.0 vs. 1.2-rc4
                            // num to text comparsion is an interesting branch.
                            return std::cmp::Ordering::Greater;
                        }

                        // FIXME: text to text comparsion is the hardest part,
                        // I just return `Ordering::Equal` currently...
                    }
                    None => {
                        if a_part.parse::<u32>().is_ok() {
                            let a_part_num = a_part.parse::<u32>().unwrap();
                            if 0 == a_part_num {
                                continue;
                            }
                        } else {
                            return std::cmp::Ordering::Less;
                        }

                        return std::cmp::Ordering::Greater;
                    }
                }
            }
            None => break,
        }
    }

    std::cmp::Ordering::Equal
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
            if name.is_some() && bucket.is_some() {
                return Ok((name.unwrap(), bucket.unwrap()));
            }
        }
    }

    Err(Error::Custom(format!("unsupported manifest path {}", path.display())).into())
}
