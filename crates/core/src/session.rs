use std::collections::HashSet;
use std::iter::FromIterator;
use std::path::Path;

use crate::{
    bucket::{BucketList, BucketUpdateContext},
    cache::CacheFile,
    config::{Config, ConfigBuilder},
    error::Fallible,
    package::{DownloadProgressContext, InstallOption, PackageList, SearchMode},
};

#[derive(Debug)]
pub struct Session {
    pub config: Config,
}

impl Session {
    pub fn init() -> Fallible<Session> {
        let config = ConfigBuilder::new().build()?;
        Ok(Session { config })
    }

    pub fn init_with<P: AsRef<Path>>(config_path: P) -> Fallible<Session> {
        let config = ConfigBuilder::new().with_path(config_path).build()?;
        Ok(Session { config })
    }

    pub fn bucket_add(&self, name: &str, repo: &str) -> Fallible<()> {
        crate::bucket::bucket_add(self, name, repo)
    }

    /// Return a list of built-in buckets, per result in form of `(name, repo)`.
    pub fn bucket_known(&self) -> Vec<(&'static str, &'static str)> {
        crate::constants::BUILTIN_BUCKET_LIST.to_vec()
    }

    pub fn bucket_list(&self) -> Fallible<BucketList> {
        crate::bucket::bucket_list(self)
    }

    pub fn bucket_remove(&self, name: &str) -> Fallible<()> {
        crate::bucket::bucket_remove(self, name)
    }

    pub fn bucket_update<F>(&mut self, cb: F) -> Fallible<()>
    where
        F: FnMut(BucketUpdateContext) + Send + 'static,
    {
        crate::bucket::bucket_update(self, cb)
    }

    pub fn cache_list(&self, query: &str) -> Fallible<Vec<CacheFile>> {
        crate::cache::cache_list(self, query)
    }

    pub fn cache_remove(&self, query: &str) -> Fallible<()> {
        crate::cache::cache_remove(self, query)
    }

    pub fn config_list(&self) -> Fallible<String> {
        crate::config::config_list(self)
    }

    pub fn config_set(&mut self, key: &str, value: &str) -> Fallible<()> {
        self.config.set(key, value)
    }

    pub fn config_unset(&mut self, key: &str) -> Fallible<()> {
        self.config.set(key, "")
    }

    pub fn package_install<F>(
        &self,
        query: &str,
        options: HashSet<InstallOption>,
        callback: F,
    ) -> Fallible<()>
    where
        F: FnMut(DownloadProgressContext) + Send + 'static,
    {
        let queries = HashSet::from_iter(query.split(' ').collect::<Vec<&str>>());
        // let _: HashSet<&str> = HashSet::from_iter(options.split(' ').collect::<Vec<&str>>());
        let packages = crate::package::resolve_packages(self, queries)?;
        crate::package::install_packages(self, packages, options, callback)?;
        Ok(())
    }

    pub fn package_list(&self, query: &str, upgradable: bool) -> Fallible<PackageList> {
        let queries = HashSet::<&str>::from_iter(query.split(' ').collect::<Vec<_>>());
        crate::package::search_installed_packages(self, queries, upgradable)
    }

    /// Available options:
    /// - `--explicit`: Turn off fuzzy search and use explicit search
    /// - `--names-only`: only search through names
    /// - `--with-binaries`: also search through binaries
    pub fn package_search(&self, query: &str, mode: &str) -> Fallible<PackageList> {
        let queries = HashSet::<&str>::from_iter(query.split(' ').collect::<Vec<_>>());
        let mode = match mode {
            "explicit" => SearchMode::Explicit,
            "unique" => SearchMode::Unique,
            "names-only" => SearchMode::FuzzyNamesOnly,
            "with-binaries" => SearchMode::FuzzyWithBinaries,
            _ => SearchMode::FuzzyDefault,
        };
        crate::package::search_available_packages(self, queries, mode)
    }
}
