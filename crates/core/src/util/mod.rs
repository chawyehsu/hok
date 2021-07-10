mod fs;
mod tokio_util;

pub use fs::*;
use once_cell::sync::Lazy;
use regex::Regex;
use regex::RegexBuilder;
use std::path::Path;
pub use tokio_util::*;

pub fn os_is_arch64() -> bool {
    match std::mem::size_of::<&char>() {
        4 => false,
        8 => true,
        _ => panic!("unexpected os arch"),
    }
}

/// FIXME: there is a wide knowledge of version comparsion,
/// And of cause I can't implement all of them at one commit, so plese fix me.
/// Perhaps https://github.com/timvisee/version-compare/issues/20 would
/// be a good reference.
pub fn compare_versions(ver_a: &String, ver_b: &String) -> std::cmp::Ordering {
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

pub fn extract_bucket_from<P: AsRef<Path> + ?Sized>(path: &P) -> Option<String> {
    static REGEX_BUCKET_NAME: Lazy<Regex> = Lazy::new(|| {
        RegexBuilder::new(r".*?[\\/]buckets[\\/](?P<bucket_name>[a-zA-Z0-9-_]+)[\\/]+.*")
            .build()
            .unwrap()
    });

    match REGEX_BUCKET_NAME.captures(path.as_ref().to_str().unwrap()) {
        Some(caps) => caps.name("bucket_name").map(|m| m.as_str().to_string()),
        None => None,
    }
}
