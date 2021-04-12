use scoop::{Scoop, app, config};

fn main() {
  let app = app::build_app();
  let matches = app.get_matches();
  let scoop = Scoop::from_cfg(config::load_cfg());

  // scoop bucket add|list|known|rm [<args>]
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
        // println!("You want add known bucket '{}'", bucket_name);
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
      drop(sub_m2);
      // Scoop::get_known_buckets();
    }
  }

  if let Some(sub_m) = matches.subcommand_matches("home") {
    println!("You want to open home of {}", sub_m.value_of("app").unwrap())
  }
}
