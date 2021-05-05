use anyhow::{anyhow, Result};
use crate::{Scoop, utils};

impl Scoop {
  pub fn cache_show(&self, app_name: Option<&str>) -> Result<()> {
    let entries = std::fs::read_dir(&self.cache_dir)?;

    for entry in entries {
      let entry = entry?;
      let fsize = entry.metadata()?.len();
      let fname = entry.file_name();
      let fname = fname.to_str().unwrap();
      let fname_split: Vec<&str> = fname.split("#").collect();

      // filter files not downloaded by Scoop
      if 2 > fname_split.len() {
        continue;
      }

      match app_name {
        Some(app_name) => {
          if app_name.eq(fname_split[0]) {
            println!("{: >6} {} ({}) {}",
              utils::filesize(fsize, true),
              fname_split[0],
              fname_split[1],
              fname
            );
          }
        },
        None => {
          println!("{: >6} {} ({}) {}",
            utils::filesize(fsize, true),
            fname_split[0],
            fname_split[1],
            fname
          );
        }
      }
    }

    Ok(())
  }

  pub fn cache_clean(&self) -> Result<()> {
    let ref cache_dir = self.cache_dir;

    if cache_dir.exists() {
      match remove_dir_all::remove_dir_contents(cache_dir) {
        Ok(()) => println!("All downloaded caches were removed."),
        Err(_e) => return Err(anyhow!("Failed to clear the caches."))
      };
    }

    Ok(())
  }

  pub fn cache_rm(&self, app_name: &str) -> Result<()> {
    match app_name {
      "*" => self.cache_clean()?,
      _ => {
        let entries: Vec<std::fs::DirEntry> = std::fs::read_dir(&self.cache_dir)?
          .filter_map(Result::ok)
          .filter(|entry| {
            let fname = entry.file_name();
            let fname = fname.to_str().unwrap();

            if app_name.ends_with("*") { // trick to support `scoop cache rm app*`
              fname.starts_with(&app_name[..app_name.len() - 1])
            } else {
              match fname.split_once("#") {
                Some((a, _b)) => a.eq(app_name),
                None => false
              }
            }
          })
          .collect();

        // nothing to do for zero entry
        if 0 == entries.len() {
          return Ok(())
        }

        for entry in entries {
          std::fs::remove_file(entry.path())?;
        }

        println!("All caches that match '{}' were removed.", app_name);
      }
    }

    Ok(())
  }
}
