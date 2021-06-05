use crate::Scoop;
use anyhow::Result;

impl<'a> Scoop<'a> {
    pub fn update_buckets(&mut self) -> Result<()> {
        for (bucket_name, bucket) in self.bucket_manager.get_buckets() {
            print!("Updating '{}' bucket...", bucket_name);

            match self.git.reset_head(bucket.path.as_path()) {
                Ok(()) => {}
                Err(e) => {
                    print!(" failed. ({})", e);
                }
            }

            println!("");
        }

        // update lastupdate
        // self.set_config("lastupdate", Utc::now()
        // .to_rfc3339_opts(SecondsFormat::Micros, false).as_str())
        Ok(())
    }
}
