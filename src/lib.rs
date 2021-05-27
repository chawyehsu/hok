pub mod cli;
pub mod bucket;
pub mod cache;
pub mod cmd;
pub mod config;
pub mod fs;
pub mod git;
pub mod http;
pub mod manifest;
pub mod search;
pub mod spdx;
pub mod utils;
pub mod update;
pub mod versions;

use bucket::BucketManager;
use cache::CacheManager;
use git::Git;
use http::Client;
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::{fs::DirEntry, io::BufReader};
use std::path::PathBuf;
use serde_json::Value;
use anyhow::{anyhow, Result};
use crate::config::Config;

#[derive(Debug)]
pub struct ScoopApp {
  pub name: String,
  pub entry: DirEntry,
  pub global: bool
}

#[derive(Debug)]
pub struct Scoop {
  pub config: Config,
  pub http: Client,
  pub bucket_manager: BucketManager,
  pub cacher: CacheManager,
  pub git: Git
}

impl Scoop {
  pub fn new(config: Config) -> Scoop {
    let http = Client::new(&config).unwrap();
    let bucket_manager = BucketManager::new(&config);
    let cacher = CacheManager::new(&config);
    let git = Git::new(&config);
    Scoop { config, http, bucket_manager, cacher, git }
  }

  pub fn dir<S: AsRef<str>>(&self, dir: S) -> PathBuf {
    self.config.get("root_path").unwrap()
      .as_str().map(PathBuf::from).unwrap().join(dir.as_ref())
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
    let mut apps: Vec<ScoopApp> = std::fs::read_dir(self.dir("apps"))?
      .filter_map(Result::ok)
      .filter(|x| !x.file_name().to_str().unwrap().starts_with("scoop"))
      .map(|e| ScoopApp {
        name: e.file_name().to_str().unwrap().to_string(),
        entry: e,
        global: false
      })
      .collect();

    let global_path = PathBuf::from(
      self.config.get("global_path").unwrap()
      .as_str().unwrap());

    if global_path.exists() {
      let global_apps: Vec<ScoopApp> = std::fs::read_dir(global_path.as_path())?
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
