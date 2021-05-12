extern crate anyhow;
extern crate remove_dir_all;

use std::process::{exit, Command};
use serde_json::Value;
use anyhow::Result;
use scoop::{cli, config, bucket, Scoop};

fn main() -> Result<()> {
  let app = cli::build_app();
  let matches = app.get_matches();
  let mut scoop = Scoop::new(config::load_cfg()?);

  // scoop bucket add|list|known|rm [<repo>]
  if let Some(sub_m) = matches.subcommand_matches("bucket") {
    if let Some(sub_m2) = sub_m.subcommand_matches("add") {
      let bucket_name = sub_m2.value_of("name").unwrap();

      if scoop.is_local_bucket(bucket_name)? {
        println!("The '{}' already exists.", bucket_name);
        exit(1);
      }

      if bucket::is_known_bucket(bucket_name) {
        let bucket_url = bucket::known_bucket_url(bucket_name).unwrap();
        scoop.clone(bucket_name, bucket_url)?;
      } else {
        match sub_m2.value_of("repo") {
          Some(repo) => {
            scoop.clone(bucket_name, repo)?;
          },
          None => {
            eprintln!("<repo> is required for unknown bucket.");
            exit(1);
          }
        }
      }
    } else if let Some(_sub_m2) = sub_m.subcommand_matches("list") {
      for b in scoop.local_buckets()? {
        println!("{}", b.name.as_str());
      }
    } else if let Some(_sub_m2) = sub_m.subcommand_matches("known") {
      for b in bucket::known_buckets() {
        println!("{}", b);
      }
    } else if let Some(sub_m2) = sub_m.subcommand_matches("rm") {
      let bucket_name = sub_m2.value_of("name").unwrap();

      if scoop.is_local_bucket(bucket_name)? {
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
      let fuzzy = sub_m.is_present("fuzzy");
      let with_binary = sub_m.is_present("binary");
      scoop.search(query, fuzzy, with_binary)?;
    }
  // scoop config list|remove
  } else if let Some(sub_m) = matches.subcommand_matches("config") {
    if let Some(_sub_m2) = sub_m.subcommand_matches("list") {
      for (key, value) in scoop.config.as_object().unwrap() {
        println!("{}: {}", key, value);
      }
    } else if let Some(sub_m2) = sub_m.subcommand_matches("remove") {
      let key = sub_m2.value_of("name").unwrap();
      scoop.set_config(key, "null")?;
    } else {
      let key = sub_m.value_of("name").unwrap();

      if let Some(value) = sub_m.value_of("value") {
        scoop.set_config(key, value)?
      } else {
        match scoop.get_config(key) {
          Some(value) => println!("{}", value.as_str().unwrap()),
          None => println!("No configration named '{}' found.", key),
        }
      }
    }
  // scoop update
  } else if let Some(_sub_m) = matches.subcommand_matches("update") {
    scoop.update_buckets()?;
  // scoop install [FLAGS] <app>...
  } else if let Some(sub_m) = matches.subcommand_matches("install") {
    todo!();
  // scoop list
  } else if let Some(_sub_m) = matches.subcommand_matches("list") {
    let brew_list_mode = scoop.config.get("brewListMode")
      .unwrap_or(&Value::Bool(false)).as_bool().unwrap();

    let apps = scoop.installed_apps()?;
    if apps.len() > 0 {
      if brew_list_mode {
        todo!();
      } else {
        println!("Installed apps:");
        for app in apps {
          let version = scoop.current_version(&app.entry)?;
          let install_info = scoop.install_info(&app.entry, &version);

          // name, version
          print!("  {} {}", app.name, version);
          // global
          if app.global {
            print!(" *global*");
          }
          // failed
          if install_info.is_err() {
            print!(" *failed*");
          }
          // hold
          let install_info = install_info?;
          if install_info.get("hold").is_some() {
            print!(" *hold*");
          }
          // bucket
          let bucket_info = install_info.get("bucket");
          if bucket_info.is_some() {
            print!(" [{}]", bucket_info.unwrap().as_str().unwrap());
          } else if install_info.get("url").is_some() {
            print!(" [{}]", install_info.get("url").unwrap().as_str().unwrap());
          }
          // arch
          // TODO
          print!("\n");
        }
      }
    }
  }

  Ok(())
}
