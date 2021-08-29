use crate::model::CacheFile;
use crate::Config;
use crate::ScoopResult;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;

/// The `CacheManager` is responsible for cache manipulation, including reading
/// and writing cache files from/to Scoop's cache directory. All cache-related
/// APIs, such as listing cache files, are exposed from this struct.
#[derive(Debug)]
pub struct CacheManager<'a> {
    /// A shared reference to Scoop's [`Config`], `CacheManager` uses it to
    /// determine its working directory, i.e. the cache directory.
    config: &'a Config,
}

impl<'a> CacheManager<'a> {
    /// Create a [`CacheManager`] with the given Scoop [`Config`]. Caller can
    /// manipulate Scoop caches by using the `CacheManager`.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// let cache_manager = CacheManger::new(&config);
    /// let cache_files = cache_manager.entries();
    /// ```
    ///
    #[inline]
    pub fn new(config: &Config) -> CacheManager {
        CacheManager { config }
    }

    /// Get all cache file entries, each entry is represented as a [`CacheFile`].
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// let cache_manager = CacheManger::new(&config);
    /// let cache_files = cache_manager.entries();
    /// ```
    ///
    /// ## Errors
    ///
    /// If the process lacks permissions to view the contents, then the function
    /// will bubble up an [`std::io::error::Error`].
    ///
    #[inline]
    pub fn entries(&self) -> ScoopResult<Vec<CacheFile>> {
        let mut res = self
            .config
            .cache_path()
            .read_dir()?
            .par_bridge()
            .filter_map(Result::ok)
            .filter_map(|de| CacheFile::new(de.path()).ok())
            .collect::<Vec<_>>();
        res.sort_by_key(|file| file.app_name());
        Ok(res)
    }

    /// Get cache file entries, which its name matching the given `app_name`.
    /// Wildcard pattern `*` is support. It will return all cache entries, the
    /// same as calling the `entries` method, when passing wildcard pattern.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// let cache_manager = CacheManger::new(&config);
    /// let python_caches = cache_manager.entries_of("python");
    /// // Passing wildcard pattern
    /// let all_caches = cache_manager.entries_of("*");
    /// // equals to:
    /// let all_caches = cache_manager.entries();
    /// ```
    ///
    /// ## Errors
    ///
    /// If the process lacks permissions to view the contents, then the function
    /// will bubble up an [`std::io::error::Error`].
    ///
    pub fn entries_of<S>(&self, app_name: S) -> ScoopResult<Vec<CacheFile>>
    where
        S: AsRef<str>,
    {
        match app_name.as_ref() {
            "*" => self.entries(),
            other => {
                let query = other.trim_end_matches("*");
                self.entries().map(|res| {
                    res.into_iter()
                        .filter(|f| f.app_name().contains(query))
                        .collect::<Vec<_>>()
                })
            }
        }
    }

    /// Remove all cache file entries.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// let cache_manager = CacheManger::new(&config);
    /// let result = cache_manager.remove_all();
    /// ```
    ///
    /// ## Errors
    ///
    /// If the process lacks permissions to delete contents, then the function
    /// will bubble up an [`std::io::error::Error`].
    ///
    #[inline]
    pub fn remove_all(&self) -> ScoopResult<()> {
        Ok(crate::util::empty_dir(self.config.cache_path())?)
    }

    /// Remove cache file entries, which its name matching the given `app_name`.
    /// Wildcard pattern `*` is support. It will remove all cache entries, the
    /// same as calling the `remove_all` method, when passing wildcard pattern.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// let cache_manager = CacheManger::new(&config);
    /// let result = cache_manager.remove("python");
    /// // Passing wildcard pattern
    /// let result = cache_manager.remove("*");
    /// // equals to:
    /// let result = cache_manager.remove_all();
    /// ```
    ///
    /// ## Errors
    ///
    /// If the process lacks permissions to delete contents, then the function
    /// will bubble up an [`std::io::error::Error`].
    ///
    #[inline]
    pub fn remove<S: AsRef<str>>(&self, app_name: S) -> ScoopResult<()> {
        match app_name.as_ref() {
            "*" => Ok(self.remove_all()?),
            _ => {
                for item in self.entries_of(app_name.as_ref())? {
                    std::fs::remove_file(item.path())?;
                }
                Ok(())
            }
        }
    }

    /// Create a new [`CacheFile`] with the given `app`, `version` and `url`.
    ///
    /// This method does not actually creates the cache file, caller should
    /// use the returned CacheFile placeholder to write data to finalize
    /// the cache creating operation.
    ///
    /// This method will always return a CacheFile placeholder for the
    /// given `app`, `version` and `url`, even it already exists. To reuse
    /// cache that already exists, caller may call the `exists` method of
    /// the CacheFile's path before writing data.
    ///
    #[inline]
    pub fn add<S: AsRef<str>>(&self, app: S, version: S, url: S) -> CacheFile {
        let filename = format!(
            "{}#{}#{}",
            app.as_ref(),
            version.as_ref(),
            crate::util::filenamify(url.as_ref())
        );
        let path = self.config.cache_path().join(filename);
        CacheFile::new(path).unwrap()
    }
}
