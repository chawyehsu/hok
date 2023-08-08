pub mod archive;
pub mod dag;
pub mod env;
pub mod fs;
pub mod git;
pub mod network;
pub mod os;
pub mod path;

/// FIXME: there is a wide knowledge of version comparsion,
/// And of cause I can't implement all of them at one commit, so plese fix me.
/// Perhaps https://github.com/timvisee/version-compare/issues/20 would
/// be a good reference.
pub fn compare_versions<S: AsRef<str>>(ver_a: S, ver_b: S) -> std::cmp::Ordering {
    let ver_a = ver_a.as_ref();
    let ver_b = ver_b.as_ref();
    let ver_a_parsed = ver_a.split(&['.', '-'][..]);
    let mut ver_b_parsed = ver_b.split(&['.', '-'][..]);
    // debug!(
    //     "ver_a_parsed: {:?}, ver_b_parsed: {:?}",
    //     ver_a_parsed, ver_b_parsed
    // );

    for a_part in ver_a_parsed {
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

    std::cmp::Ordering::Equal
}
