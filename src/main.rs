extern crate remove_dir_all;

use scoop::{Scoop, app, config};

fn main() {
  let app = app::build_app();
  let matches = app.get_matches();
  let scoop = Scoop::from_cfg(config::load_cfg());

  // scoop bucket add|list|known|rm [<repo>]
  if let Some(sub_m) = matches.subcommand_matches("bucket") {
    if let Some(sub_m2) = sub_m.subcommand_matches("add") {
      let bucket_name = sub_m2.value_of("name").unwrap();

      if scoop.is_added_bucket(bucket_name) {
        println!("The '{}' already exists.", bucket_name);
        return;
      }

      if Scoop::is_known_bucket(bucket_name) {
        let bucket_url = Scoop::get_known_bucket_url(bucket_name);
        scoop.clone(bucket_name, bucket_url);
      } else {
        let bucket_url = sub_m2.value_of("repo")
          .expect("<repo> is required for unknown bucket");
        scoop.clone(bucket_name, bucket_url);
      }
    }

    if let Some(sub_m2) = sub_m.subcommand_matches("list") {
      drop(sub_m2);
      scoop.buckets();
    }

    if let Some(sub_m2) = sub_m.subcommand_matches("known") {
      drop(sub_m2);
      Scoop::get_known_buckets();
    }

    if let Some(sub_m2) = sub_m.subcommand_matches("rm") {
      let bucket_name = sub_m2.value_of("name").unwrap();

      if scoop.is_added_bucket(bucket_name) {
        let bucket_dir = scoop.buckets_dir.join(bucket_name);
        if bucket_dir.exists() {
          match remove_dir_all::remove_dir_all(bucket_dir) {
            Ok(()) => {},
            Err(e) => panic!("failed to remove '{}' bucket. {}", bucket_name, e)
          };
        }
      } else {
        println!("The '{}' bucket not found.", bucket_name);
      }
    }
  }

  if let Some(sub_m) = matches.subcommand_matches("home") {
    println!("You want to open home of {}", sub_m.value_of("app").unwrap())
  }
}
