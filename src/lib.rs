pub mod cli;
pub mod bucket;
pub mod cache;
pub mod config;
pub mod fs;
pub mod git;
pub mod manifest;
pub mod search;
pub mod spdx;
pub mod utils;
pub mod update;
pub mod versions;

use dirs;
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::{env, fs::DirEntry, io::BufReader};
use std::path::PathBuf;
use serde_json::Value;
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct ScoopApp {
  pub name: String,
  pub entry: DirEntry,
  pub global: bool
}

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

  pub fn install_info(&self, app: &DirEntry, version: &String) -> Result<Value> {
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

  pub fn installed_apps(&self) -> Result<Vec<ScoopApp>> {
    let mut apps: Vec<ScoopApp> = std::fs::read_dir(&self.apps_dir)?
      .filter_map(Result::ok)
      .filter(|x| !x.file_name().to_str().unwrap().starts_with("scoop"))
      .map(|e| ScoopApp {
        name: e.file_name().to_str().unwrap().to_string(),
        entry: e,
        global: false
      })
      .collect();

    if self.global_dir.exists() {
      let global_apps: Vec<ScoopApp> = std::fs::read_dir(&self.global_dir)?
        .filter_map(Result::ok)
        .map(|e| ScoopApp {
          name: e.file_name().to_str().unwrap().to_string(),
          entry: e,
          global: false
        })
        .collect();
      apps.extend(global_apps);
    }

    Ok(apps)
  }

  pub fn parse_app<'a>(&self, app: &'a str) -> (&'a str, Option<&'a str>, Option<&'a str>) {
    static RE: Lazy<Regex> = Lazy::new(|| {
      RegexBuilder::new(r"(?:(?P<bucket>[a-zA-Z0-9-]+)/)?(?P<app>.*.json$|[a-zA-Z0-9-_.]+)(?:@(?P<version>.*))?")
      .build().unwrap()
    });

    let caps = RE.captures(app).unwrap();

    let app: &str = caps.name("app").unwrap().as_str();
    let bucket: Option<&str> = match caps.name("bucket") {
      Some(m) => Some(m.as_str()),
      None => None
    };
    let version: Option<&str> = match caps.name("version") {
      Some(m) => Some(m.as_str()),
      None => None
    };

    (app, bucket, version)
  }

  pub fn is_installed(&self, app: &str) -> bool {
    let app = app.trim_end_matches(".json")
      .split(&['/', '\\'][..]).last().unwrap().to_string();

    let apps: Vec<String> = self.installed_apps().unwrap()
      .into_iter().map(|a| a.name).collect();

    // FIXME: need an alternative searching algo
    apps.contains(&app)
  }
}
