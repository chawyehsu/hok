use crate::Scoop;
use anyhow::Result;
use chrono::{Utc, SecondsFormat};

impl Scoop {
  pub fn update_buckets(&mut self) -> Result<()> {
    let buckets = self.get_local_buckets_entry()?;

    for bucket in buckets {
      let bucket_name = bucket.file_name().to_str().unwrap().to_owned();
      print!("Updating '{}' bucket...", bucket_name);

      match self.reset_head(bucket.path()) {
        Ok(()) => {},
        Err(e) => {
          print!(" failed. ({})", e);
        }
      }

      println!("");
    }

    // update lastupdate
    self.set_config("lastupdate", Utc::now()
      .to_rfc3339_opts(SecondsFormat::Micros, false).as_str())
      .unwrap();

    Ok(())
  }
}
