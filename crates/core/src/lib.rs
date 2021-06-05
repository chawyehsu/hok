pub mod apps;
pub mod bucket;
pub mod cache;
pub mod config;
pub mod error;
pub mod fs;
pub mod git;
pub mod http;
pub mod manifest;
pub mod persist;
pub mod search;
pub mod spdx;
pub mod sys;
pub mod utils;

use std::path::PathBuf;

use apps::AppManager;
use bucket::BucketManager;
use cache::CacheManager;
use config::Config;
use git::GitTool;
use http::Client;

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
