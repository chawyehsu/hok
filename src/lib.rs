pub mod app;
pub mod bucket;
pub mod cache;
pub mod config;
pub mod git;
pub mod manifest;
pub mod search;
pub mod utils;
pub mod update;
pub mod versions;

use dirs;
use std::{env, fs::DirEntry, io::BufReader};
use std::path::PathBuf;
use serde_json::Value;
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct Scoop {
  pub config: Value,
  pub root_dir: PathBuf,
  pub cache_dir: PathBuf,
  pub global_dir: PathBuf,
  pub apps_dir: PathBuf,
  pub buckets_dir: PathBuf,
  pub modules_dir: PathBuf,
  pub persist_dir: PathBuf,
  pub shims_dir: PathBuf
}

impl Scoop {
  pub fn new(config: Value) -> Scoop {
    let root_dir: PathBuf = config["rootPath"]
      .as_str()
      .map(PathBuf::from)
      .unwrap_or_else(
        || env::var_os("SCOOP")
          .map(PathBuf::from).filter(|p| p.is_absolute())
          .unwrap_or_else(
            || dirs::home_dir().map(|p| p.join("scoop")).unwrap()
        )
      );

    let cache_dir: PathBuf = config["cachePath"]
      .as_str()
      .map(PathBuf::from)
      .unwrap_or_else(
        || env::var_os("SCOOP_CACHE")
          .map(PathBuf::from).filter(|p| p.is_absolute())
          .unwrap_or_else(
            || dirs::home_dir().map(|p| p.join("scoop\\cache")).unwrap()
        )
      );

    let global_dir: PathBuf = config["cachePath"]
      .as_str()
      .map(PathBuf::from)
      .unwrap_or_else(
        || env::var_os("SCOOP_GLOBAL")
          .map(PathBuf::from).filter(|p| p.is_absolute())
          .unwrap_or_else(
            || env::var_os("ProgramData")
              .map(PathBuf::from).map(|p| p.join("scoop")).unwrap()
        )
      );

    let apps_dir: PathBuf = root_dir.join("apps");
    let buckets_dir: PathBuf = root_dir.join("buckets");
    let modules_dir: PathBuf = root_dir.join("modules");
    let persist_dir: PathBuf = root_dir.join("persist");
    let shims_dir: PathBuf = root_dir.join("shims");

    Scoop {
      config, root_dir, cache_dir, global_dir, apps_dir,
      buckets_dir, modules_dir, persist_dir, shims_dir
    }
  }

  fn install_info(&self, app: &DirEntry, version: &String) -> Result<Value> {
    let install_json = app.path()
      .join(version).join("install.json");

    let file = std::fs::File::open(install_json.as_path())?;
    let reader = BufReader::new(file);

    match serde_json::from_reader(reader) {
      Ok(m) => return Ok(m),
      Err(_e) => {
        let msg = format!("Failed to parse install.json of app '{}' (version: {}).",
        app.file_name().to_str().unwrap().to_owned(),
        version);
        return Err(anyhow!(msg));
      }
    }
  }

  pub fn installed_apps(&self) -> Result<()> {
    let brew_list_mode = self.config.get("brewListMode")
      .unwrap_or(&Value::Bool(false)).as_bool().unwrap();
    let mut apps: Vec<(DirEntry, bool)> = std::fs::read_dir(&self.apps_dir)?
      .filter_map(Result::ok)
      .filter(|x| !x.file_name().to_str().unwrap().starts_with("scoop"))
      .map(|e| (e, false))
      .collect();

    if self.global_dir.exists() {
      let global_apps: Vec<(DirEntry, bool)> = std::fs::read_dir(&self.global_dir)?
        .filter_map(Result::ok)
        .map(|e| (e, true))
        .collect();
      apps.extend(global_apps);
    }

    if apps.len() > 0 {
      if brew_list_mode {
        todo!();
      } else {
        println!("Installed apps:");
        for (app, global) in apps {
          let name = app.file_name().to_str().unwrap().to_owned();
          let version = self.current_version(&app)?;
          let install_info = self.install_info(&app, &version);

          // name, version
          print!("  {} {}", name, version);
          // global
          if global {
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

    Ok(())
  }
}
