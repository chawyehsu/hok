use chrono::{SecondsFormat, Utc};
use scoop_core::Scoop;

pub fn cmd_update(_: &clap::ArgMatches, scoop: &mut Scoop) {
    for (bucket_name, bucket) in scoop.bucket_manager.get_buckets() {
        print!("Updating '{}' bucket...", bucket_name);

        match scoop.git.reset_head(bucket.path.as_path()) {
            Ok(()) => {}
            Err(e) => {
                print!(" failed. ({})", e);
            }
        }

        println!("");
    }

    // update lastupdate
    let time = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false);
    scoop
        .config
        .set("lastupdate", time.as_str())
        .unwrap()
        .save();
}
