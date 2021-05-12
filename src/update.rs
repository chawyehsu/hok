use crate::Scoop;
use anyhow::Result;
use chrono::{Utc, SecondsFormat};

impl Scoop {
  pub fn update_buckets(&mut self) -> Result<()> {
    let buckets = self.local_buckets()?;

    for bucket in buckets.into_iter() {
      print!("Updating '{}' bucket...", bucket.name.as_str());

      match self.reset_head(bucket.entry.unwrap().path()) {
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
