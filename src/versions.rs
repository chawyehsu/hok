use anyhow::{anyhow, Result};
use std::fs::DirEntry;

use crate::Scoop;

impl Scoop {
  // FIXME: there is a wide knowledge of version comparsion,
  // And of cause I can't implement all of them at one commit, so plese fix me.
  // Perhaps https://github.com/timvisee/version-compare/issues/20 would
  // be a good reference.
  fn compare_versions(ver_a: &String, ver_b: &String) -> std::cmp::Ordering {
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
            },
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
        },
        None => break
      }
    }

    std::cmp::Ordering::Equal
  }

  fn versions(&self, app_dir: &DirEntry) -> Result<Vec<String>> {
    let mut installed_versions: Vec<String> = std::fs::read_dir(app_dir.path())?
    .filter_map(Result::ok)
    .filter(|x| x.metadata().unwrap().is_dir() &&
      x.file_name().to_str().unwrap().to_owned().ne("current"))
    .map(|y| y.file_name().to_str().unwrap().to_string())
    .collect();

    if installed_versions.len() > 1 {
      installed_versions.sort_unstable_by(Self::compare_versions);
    }

    // print!(" {:?}", installed_versions);
    // println!("");
    Ok(installed_versions)
  }

  pub fn current_version(&self, app_dir: &DirEntry) -> Result<String> {
    let versions = self.versions(app_dir)?;
    let app_name = app_dir.file_name().to_str().unwrap().to_owned();

    if versions.is_empty() {
      return Err(anyhow!(format!("Faild to find any version of app '{}'", app_name)));
    }

    Ok(versions.last().unwrap().to_owned())
  }
}
