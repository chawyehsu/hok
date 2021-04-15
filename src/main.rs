extern crate remove_dir_all;
use scoop::*;

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
    } else if let Some(sub_m2) = sub_m.subcommand_matches("list") {
      drop(sub_m2);
      scoop.buckets();
    } else if let Some(sub_m2) = sub_m.subcommand_matches("known") {
      drop(sub_m2);
      Scoop::get_known_buckets();
    } else if let Some(sub_m2) = sub_m.subcommand_matches("rm") {
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
  // scoop cache show|rm [<app>]
  } else if let Some(sub_m) = matches.subcommand_matches("cache") {
    let cache_dir = &scoop.cache_dir;

    if let Some(sub_m2) = sub_m.subcommand_matches("rm") {
      if let Some(app_name) = sub_m2.value_of("app") {
        if app_name.eq("*") {
          if cache_dir.exists() {
            match remove_dir_all::remove_dir_contents(cache_dir) {
              Ok(()) => println!("All downloaded caches was removed."),
              Err(e) => panic!("failed to clear the caches. {}", e)
            };
          }
        } else {
          let app_cache_files =
            std::fs::read_dir(cache_dir)
              .unwrap()
              .map(|p| p.unwrap())
              .filter(|p|
                app_name.eq(
                  p
                    .file_name()
                    .to_str()
                    .unwrap()
                    .split_once("#")
                    .unwrap()
                    .0
                )
              );

          for f in app_cache_files {
            match std::fs::remove_file(f.path()) {
              Ok(()) => println!("All caches of app '{}' was removed.", app_name),
              Err(e) => panic!("failed to remove caches of app '{}'. {}", app_name, e)
            }
          }
        }
      } else if sub_m2.is_present("all") {
        if cache_dir.exists() {
          match remove_dir_all::remove_dir_contents(cache_dir) {
            Ok(()) => println!("All downloaded caches was removed."),
            Err(e) => panic!("failed to clear the caches. {}", e)
          };
        }
      }
    } else {
      let cache_files =
        std::fs::read_dir(cache_dir)
          .unwrap()
          .map(|p| p.unwrap())
          ;

      if let Some(sub_m2) = sub_m.subcommand_matches("show") {
        if let Some(app_name) = sub_m2.value_of("app") {
          let app_cache_files = cache_files.filter(
            |c| app_name.eq(c
              .file_name()
              .into_string()
              .unwrap()
              .split_once("#")
              .unwrap()
              .0)
          );

          for f in app_cache_files {
            let fmeta = std::fs::metadata(f.path()).unwrap();
            let ff = f.file_name().into_string().unwrap();
            let fname: Vec<&str> = ff.split("#").collect();

            println!("{: >6} {} ({}) {}",
              utils::filesize(fmeta.len(), true),
              fname[0],
              fname[1],
              ff
            );
          }
        } else {
          for f in cache_files {
            let fmeta = std::fs::metadata(f.path()).unwrap();
            let ff = f.file_name().into_string().unwrap();
            let fname: Vec<&str> = ff.split("#").collect();

            println!("{: >6} {} ({}) {}",
              utils::filesize(fmeta.len(), true),
              fname[0],
              fname[1],
              ff
            );
          }
        }
      } else {
        for f in cache_files {
          let fmeta = std::fs::metadata(f.path()).unwrap();
          let ff = f.file_name().into_string().unwrap();
          let fname: Vec<&str> = ff.split("#").collect();

          println!("{: >4} {} ({}) {}",
            utils::filesize(fmeta.len(), true),
            fname[0],
            fname[1],
            ff
          );
        }
      }
    }
  } else if let Some(sub_m) = matches.subcommand_matches("home") {
    // println!("You want to open home of {}", sub_m.value_of("app").unwrap())
  }

  // println!("{:?}", scoop);
}
