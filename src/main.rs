extern crate anyhow;
extern crate remove_dir_all;

use std::process::Command;

use anyhow::Result;
use scoop::*;

fn main() -> Result<()> {
  let app = app::build_app();
  let matches = app.get_matches();
  let mut scoop = Scoop::from_cfg(config::load_cfg());

  // scoop bucket add|list|known|rm [<repo>]
  if let Some(sub_m) = matches.subcommand_matches("bucket") {
    if let Some(sub_m2) = sub_m.subcommand_matches("add") {
      let bucket_name = sub_m2.value_of("name").unwrap();

      if scoop.is_added_bucket(bucket_name) {
        println!("The '{}' already exists.", bucket_name);
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
    if let Some(sub_m2) = sub_m.subcommand_matches("rm") {
      if let Some(app_name) = sub_m2.value_of("app") {
        scoop.cache_rm(app_name)?;
      } else if sub_m2.is_present("all") {
        scoop.cache_clean()?;
      }
    } else {
      if let Some(sub_m2) = sub_m.subcommand_matches("show") {
        scoop.cache_show(sub_m2.value_of("app"))?;
      } else {
        scoop.cache_show(None)?;
      }
    }
  // scoop home <app>
  } else if let Some(sub_m) = matches.subcommand_matches("home") {
    if let Some(app_name) = sub_m.value_of("app") {
      // find manifest and parse it
      match scoop.manifest(app_name) {
        Some(manifest) => {
          if let Some(url) = manifest.get("homepage") {
            let url = std::ffi::OsStr::new(url.as_str().unwrap());
            Command::new("cmd").arg("/C").arg("start").arg(url).spawn()?;
          } else {
            println!("Could not find homepage in manifest for '{}'.", app_name);
          }
        },
        None => {
          println!("Could not find manifest for '{}'.", app_name);
        }
      }
    }
  // scoop search <query>
  } else if let Some(sub_m) = matches.subcommand_matches("search") {
    if let Some(query) = sub_m.value_of("query") {
      scoop.search(query)?;
    }
  } else if let Some(sub_m) = matches.subcommand_matches("config") {
    if let Some(sub_m2) = sub_m.subcommand_matches("rm") {
      let key = sub_m2.value_of("name").unwrap();
      scoop.set_config(key, "null");
    } else {
      let key = sub_m.value_of("name").unwrap();

      if let Some(value) = sub_m.value_of("value") {
        scoop.set_config(key, value)
      } else {
        let value = scoop.get_config(key);
        match value.as_str() {
          "null" => println!("No configration named '{}' found.", key),
          _ => println!("{}", value),
        }
      }
    }
  }

  // println!("{:?}", scoop);
  Ok(())
}
