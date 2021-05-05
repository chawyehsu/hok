use anyhow::Result;
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
}
