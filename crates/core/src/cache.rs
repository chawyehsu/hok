use crate::{error::ScoopResult, Config};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use std::{
    path::{Path, PathBuf},
    result,
};

/// An entry represents a cache file downloaded and managed by Scoop.
#[derive(Debug)]
pub struct CacheEntry {
    path: PathBuf,
}

impl CacheEntry {
    /// Create a Scoop [`CacheEntry`] with the given PathBuf.
    ///
    /// This constructor is marked as private, since we don't want any caller
    /// outside the [`CacheManager`] to create new CacheEntry directly.
    #[inline]
    fn new(path: PathBuf) -> CacheEntry {
        CacheEntry { path }
    }

    /// Get the `app` name of this Scoop [`CacheEntry`].
    #[inline]
    pub fn app_name(&self) -> String {
        self.path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .split_once("#")
            .unwrap()
            .0
            .to_string()
    }

    /// Get the filename of this Scoop [`CacheEntry`].
    #[inline]
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    /// Get the filepath of this Scoop [`CacheEntry`].
    #[inline]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Get the tmp download path of this Scoop [`CacheEntry`].
    #[inline]
    pub fn tmp_path(&self) -> PathBuf {
        let mut temp = self.path.as_path().as_os_str().to_os_string();
        temp.push(".download");
        PathBuf::from(temp)
    }

    /// Get the file size of this Scoop [`CacheEntry`].
    #[inline]
    pub fn size(&self) -> u64 {
        self.path.metadata().unwrap().len()
    }

    /// Get the app `version` of this Scoop [`CacheEntry`].
    #[inline]
    pub fn version(&self) -> String {
        self.path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .split_once("#")
            .unwrap()
            .1
            .split_once("#")
            .unwrap()
            .0
            .to_string()
    }
}

#[derive(Debug)]
pub struct CacheManager<'a> {
    config: &'a Config,
    working_dir: PathBuf,
}

impl<'a> CacheManager<'a> {
    /// Create a Scoop [`CacheManager`] with the given PathBuf. The given
    /// PathBuf will be the working directory of this CacheManager.
    #[inline]
    pub fn new(config: &Config) -> CacheManager {
        let working_dir = config.cache_path.clone();
        CacheManager {
            config,
            working_dir,
        }
    }

    /// Get all cache files representing as [`CacheEntry`].
    #[inline]
    pub fn get_all(&self) -> ScoopResult<Vec<CacheEntry>> {
        // regex to match valid named cache files:
        // "app#version#filenamified_url"
        static RE: Lazy<Regex> = Lazy::new(|| {
            RegexBuilder::new(r"(?P<app>[0-9a-zA-Z-_.]+)#(?P<version>[0-9a-zA-Z-.]+)#(?P<url>.*)")
                .build()
                .unwrap()
        });

        let entries = self
            .working_dir
            .read_dir()?
            .filter_map(result::Result::ok)
            .filter(|de| RE.is_match(de.file_name().to_string_lossy().as_ref()))
            .map(|de| CacheEntry::new(de.path()))
            .collect();

        Ok(entries)
    }

    /// Get cache files, which its name matching the given `pattern`,
    /// representing as [`CacheEntry`].
    #[inline]
    pub fn get<T: AsRef<str>>(&self, pattern: T) -> ScoopResult<Vec<CacheEntry>> {
        let all_cache_items = self.get_all();

        match pattern.as_ref() {
            "*" => all_cache_items,
            query => Ok(all_cache_items?
                .into_iter()
                .filter(|ce| ce.app_name().starts_with(query.trim_end_matches("*")))
                .collect()),
        }
    }

    /// Remove all cache files.
    #[inline]
    pub fn remove_all(&self) -> ScoopResult<()> {
        Ok(crate::fs::empty_dir(&self.working_dir)?)
    }

    /// Remove cache files, which its name matching the given `pattern`.
    /// (wildcard `*` pattern is support)
    #[inline]
    pub fn remove<T: AsRef<str>>(&self, app_name: T) -> ScoopResult<()> {
        match app_name.as_ref() {
            "*" => Ok(self.remove_all()?),
            _ => {
                for item in self.get(app_name.as_ref())? {
                    std::fs::remove_file(item.path())?;
                }
                Ok(())
            }
        }
    }

    /// Create a new [`CacheEntry`] with the given `app`, `version` and `url`.
    ///
    /// This method does not actually creates the cache file, caller should
    /// use the returned CacheEntry placeholder to write data to finalize
    /// the cache creating operation.
    ///
    /// This method will always return a CacheEntry placeholder for the
    /// given `app`, `version` and `url`, even it already exists. To reuse
    /// cache that already exists, caller may call the `exists` method of
    /// the CacheEntry's path before writing data.
    ///
    #[inline]
    pub fn add<S: AsRef<str>>(&self, app: S, version: S, url: S) -> CacheEntry {
        let filename = format!(
            "{}#{}#{}",
            app.as_ref(),
            version.as_ref(),
            crate::utils::filenamify(url.as_ref())
        );
        let path = self.working_dir.join(filename);
        CacheEntry::new(path)
    }
}
