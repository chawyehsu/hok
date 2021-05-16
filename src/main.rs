extern crate anyhow;
extern crate remove_dir_all;

use std::process::{exit, Command};
use serde_json::Value;
use anyhow::Result;
use scoop::{cli, config, bucket, utils, manifest, Scoop};

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
        println!("{}", b.0.as_str());
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
        match scoop.cache_remove(app_name) {
          Ok(()) => {
            println!("All caches that match '{}' were removed.", app_name);
            exit(0);
          },
          Err(_e) => {
            eprintln!("Failed to remove '{}' caches.", app_name);
            exit(1);
          }
        }
      } else if sub_m2.is_present("all") {
        match scoop.cache_clean() {
          Ok(()) => {
            println!("All download caches were removed.");
            exit(0);
          },
          Err(_e) => {
            eprintln!("Failed to clear caches.");
            exit(1);
          }
        }
      }
    } else {
      let cache_items = scoop.cache_get_all()?;
      let mut total_size: u64 = 0;
      let total_count = cache_items.len();

      if let Some(sub_m2) = sub_m.subcommand_matches("show") {
        if let Some(app) = sub_m2.value_of("app") {
          let mut filter_size: u64 = 0;
          let mut filter_count: u64 = 0;
          for sci in cache_items {
            if sci.app.contains(app) {
              filter_size = filter_size + sci.size;
              filter_count = filter_count + 1;
              println!("{: >6} {} ({}) {}",
                utils::filesize(sci.size, true),
                sci.app,
                sci.version,
                sci.filename
              );
            }
          }
          if filter_count > 0 {
            println!();
          }
          println!("Total: {} files, {}",
            filter_count, utils::filesize(filter_size, true));
          exit(0);
        }
      }

      for sci in cache_items {
        total_size = total_size + sci.size;
        println!("{: >6} {} ({}) {}",
          utils::filesize(sci.size, true),
          sci.app,
          sci.version,
          sci.filename
        );
      }
      if total_count > 0 {
        println!();
      }
      println!("Total: {} files, {}",
        total_count, utils::filesize(total_size, true));
      exit(0);
    }
  // scoop home <app>
  } else if let Some(sub_m) = matches.subcommand_matches("home") {
    if let Some(app_name) = sub_m.value_of("app") {
      // find local manifest and parse it
      match scoop.find_local_manifest(app_name)? {
        Some(manifest) => {
          if let Some(url) = manifest.json.get("homepage") {
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
  // scoop update
  } else if let Some(sub_m) = matches.subcommand_matches("info") {
    let app = sub_m.value_of("app").unwrap();
    match scoop.find_local_manifest(app) {
      Ok(Some(manifest)) => {
        // Name
        println!("Name: {}", manifest.app);
        // Bucket
        if manifest.bucket.is_some() {
          println!("Bucket: {}", manifest.bucket.unwrap());
        }
        // Description
        if manifest.json.get("description").is_some() {
          println!("Description: {}", manifest.json.get("description").unwrap().as_str().unwrap());
        }
        // Version
        println!("Version: {}", manifest.version);
        // Homepage
        if manifest.json.get("homepage").is_some() {
          println!("Website: {}", manifest.json.get("homepage").unwrap().as_str().unwrap());
        }
        // License
        if manifest.license.is_some() {
          let licenses = manifest.license.unwrap();

          if licenses.len() == 1 {
            print!("License:");
            let pair = licenses.first().unwrap();
            match pair.1.as_ref() {
              Some(url) => print!(" {} ({})\n", pair.0, url),
              None => print!(" {}\n", pair.0)
            }
          } else {
            println!("License:");
            for pair in licenses {
              match pair.1 {
                Some(url) => println!("  {} ({})", pair.0, url),
                None => println!("  {}", pair.0)
              }
            }
          }
        }
        // Manifest
        match manifest.kind {
          manifest::ManifestKind::Local(path) => {
            println!("Manifest: \n  {}", path.to_str().unwrap());
          },
          manifest::ManifestKind::Remote(_url) => {} // FIXME
        }
        // Binaries
        match manifest.json.get("bin") {
          Some(Value::String(single)) => {
            println!("Binaries: \n  {}", single);
          },
          Some(Value::Array(_multiple)) => {
            println!("Binaries:");
            // for s in multiple {
            //   match s {
            //     Value::String(s) =>
            //   }
            // }
          },
          _ => {} // no-op
        }

        exit(0);
      },
      Ok(None) => {
        eprintln!("Could not find manifest for '{}'", app);
        exit(1);
      },
      Err(e) => {
        eprintln!("Failed to operate. ({})", e);
        exit(1);
      }
    }
  // scoop install [FLAGS] <app>...
  } else if let Some(_sub_m) = matches.subcommand_matches("install") {
    todo!();
    // let apps: Vec<_> = sub_m.values_of("app").unwrap().collect();

    // let app_count = apps.len();
    // for app in apps {
    //   let app_parsed = scoop.parse_app(app);

    //   let app_is_installed = scoop.is_installed(app_parsed.0);

    //   if app_count == 1 && app_is_installed && app_parsed.2.is_none() {

    //   }

    //   println!("{:?}", app_parsed);
    // }
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
