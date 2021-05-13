use crate::Scoop;
use anyhow::Result;
use chrono::{Utc, SecondsFormat};

impl Scoop {
  pub fn update_buckets(&mut self) -> Result<()> {
    for bucket in self.local_buckets()? {
      print!("Updating '{}' bucket...", bucket.0.as_str());

      match self.reset_head(bucket.1.path()) {
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
