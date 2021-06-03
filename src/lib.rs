pub mod apps;
pub mod cli;
pub mod bucket;
pub mod cache;
pub mod cmd;
pub mod config;
pub mod error;
pub mod fs;
pub mod git;
pub mod http;
pub mod log;
pub mod manifest;
pub mod persist;
pub mod search;
pub mod spdx;
pub mod sys;
pub mod utils;
pub mod update;

use std::path::PathBuf;

use apps::AppsManager;
use bucket::BucketManager;
use cache::CacheManager;
use config::Config;
use git::GitTool;
use http::Client;

#[derive(Debug)]
pub struct Scoop {
  pub config: Config,
  pub http: Client,
  pub apps_manager: AppsManager,
  pub bucket_manager: BucketManager,
  pub cacher: CacheManager,
  pub git: GitTool,
}

impl Scoop {
  pub fn new(config: Config) -> Scoop {
    let http = Client::new(&config).unwrap();
    let apps_manager = AppsManager::new(&config);
    let bucket_manager = BucketManager::new(&config);
    let cacher = CacheManager::new(&config);
    let git = GitTool::new(&config);
    Scoop { config, http, apps_manager, bucket_manager, cacher, git }
  }

  pub fn dir<S: AsRef<str>>(&self, dir: S) -> PathBuf {
    self.config.get("root_path").unwrap()
      .as_str().map(PathBuf::from).unwrap().join(dir.as_ref())
  }
}
