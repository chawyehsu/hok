use chrono::{SecondsFormat, Utc};
use scoop_core::{BucketManager, Config};

pub fn cmd_update(_: &clap::ArgMatches, config: &mut Config) {
    let bucket_manager = BucketManager::new(config);

    bucket_manager.buckets().iter().for_each(|(name, bucket)| {
        print!("Updating '{}' bucket...", name);
        match bucket.update() {
            Ok(()) => {}
            Err(e) => {
                print!(" failed. ({})", e);
            }
        }
        println!("");
    });

    // update lastupdate
    let time = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false);
    config.set("lastupdate", time.as_str()).unwrap().save();
}
