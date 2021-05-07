use crate::Scoop;
use anyhow::Result;

impl Scoop {
  pub fn update_buckets(&self) -> Result<()> {
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

    Ok(())
  }
}
