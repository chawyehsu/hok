#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod apps;
pub mod bucket;
mod cache;
mod config;
mod error;
pub mod fs;
mod git;
mod http;
pub mod manifest;
mod persist;
mod search;
mod spdx;
pub mod sys;
pub mod utils;

use std::path::PathBuf;

pub use apps::AppManager;
pub use bucket::BucketManager;
pub use cache::CacheManager;
pub use config::Config;
pub use error::{Error, Result};
use git::GitTool;
use http::Client;
pub use persist::PersistManager;
pub use spdx::SPDX;

#[derive(Debug)]
pub struct Scoop<'a> {
    pub config: &'a mut Config,

    pub app_manager: AppManager,
    pub bucket_manager: BucketManager,
    pub cache_manager: CacheManager,

    pub git: GitTool,
    pub http: Client,
}

impl<'a> Scoop<'a> {
    pub fn new(config: &'a mut Config) -> Scoop<'a> {
        let apps_dir = config.root_path.join("apps");
        let buckets_dir = config.root_path.join("buckets");
        let cache_dir = config.cache_path.to_path_buf();

        let app_manager = AppManager::new(apps_dir);
        let bucket_manager = BucketManager::new(buckets_dir);
        let cache_manager = CacheManager::new(cache_dir);

        let git = GitTool::new(&config);
        let http = Client::new(&config).unwrap();

        Scoop {
            config,
            app_manager,
            bucket_manager,
            cache_manager,
            git,
            http,
        }
    }

    pub fn dir<S: AsRef<str>>(&self, dir: S) -> PathBuf {
        self.config.root_path.join(dir.as_ref())
    }
}
