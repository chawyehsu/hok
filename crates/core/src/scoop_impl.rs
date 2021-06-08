use crate::{
    error::Result,
    git::GitTool,
    http::Client,
    manifest::Manifest,
    search::{travel_manifest, Matches},
    AppManager, BucketManager, CacheManager, Config,
};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

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

    /// Find and return local manifest represented as [`Manifest`], using given
    /// `pattern`.
    ///
    /// bucket name prefix is support, for example:
    /// ```
    /// let manifest = find_local_manifest("main/gcc");
    /// ```
    pub fn find_local_manifest<T: AsRef<str>>(&self, pattern: T) -> Result<Option<Manifest>> {
        // Detect given pattern whether having bucket name prefix
        let (bucket_name, app_name) = match pattern.as_ref().contains("/") {
            true => {
                let (a, b) = pattern.as_ref().split_once("/").unwrap();
                (Some(a), b)
            }
            false => (None, pattern.as_ref()),
        };

        match bucket_name {
            Some(bucket_name) => {
                let bucket = self.bucket_manager.get_bucket(bucket_name).unwrap();
                let manifest_path = bucket.manifest_dir().join(format!("{}.json", app_name));
                match manifest_path.exists() {
                    true => Ok(Some(Manifest::from_path(&manifest_path)?)),
                    false => Ok(None),
                }
            }
            None => {
                for (_, bucket) in self.bucket_manager.get_buckets() {
                    let manifest_path = bucket.manifest_dir().join(format!("{}.json", app_name));
                    if manifest_path.exists() {
                        return Ok(Some(Manifest::from_path(&manifest_path)?));
                    }
                }

                Ok(None)
            }
        }
    }

    pub fn search(&self, query: &str, search_bin: bool) -> Result<Vec<Matches>> {
        // Load all local buckets
        let buckets = self.bucket_manager.get_buckets();

        let mut matches: Vec<Matches> = Vec::new();

        buckets.iter().for_each(|(bucket_name, bucket)| {
            let manifests = bucket.available_manifests().unwrap();
            let search_matches = Arc::new(Mutex::new(Vec::new()));

            manifests.par_iter().for_each(|manifest_path| {
                match travel_manifest(query, search_bin, manifest_path).unwrap() {
                    Some(sm) => search_matches.lock().unwrap().push(sm),
                    None => {}
                }
            });

            let mut collected = search_matches.lock().unwrap().to_vec();
            collected.sort_by_key(|s| s.name.to_string());

            matches.push(Matches {
                bucket: bucket_name.to_string(),
                collected,
            });
        });

        matches.sort_by_key(|k| k.bucket.to_string());

        Ok(matches)
    }
}
