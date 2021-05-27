use std::{fs::DirEntry, path::PathBuf};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use anyhow::Result;
use crate::config::Config;

pub struct ScoopCacheItem {
  pub app: String,
  pub entry: DirEntry,
  pub filename: String,
  pub size: u64,
  pub version: String
}

#[derive(Debug)]
pub struct CacheManager {
  cache_dir: PathBuf
}

impl CacheManager {
  pub fn new(config: &Config) -> CacheManager {
    let cache_dir = PathBuf::from(
      config.get("cache_path").unwrap().as_str().unwrap()
    );

    CacheManager { cache_dir }
  }

  /// Collect all cache files represented as [`ScoopCacheItem`]
  pub fn get_all(&self) -> Result<Vec<ScoopCacheItem>> {
    static RE: Lazy<Regex> = Lazy::new(|| {
      RegexBuilder::new(r"(?P<app>[a-zA-Z0-9-_.]+)#(?P<version>[a-zA-Z0-9-.]+)#(?P<url>.*)")
      .build().unwrap()
    });

    let entries = std::fs::read_dir(self.cache_dir.as_path())?
      .filter_map(Result::ok)
      .filter(|de| RE.is_match(de.file_name().to_str().unwrap()))
      .map(|entry| {
        let filename = entry.file_name().into_string().unwrap();
        let size = entry.metadata().unwrap().len();
        let (a, b) = filename.split_once("#").unwrap();
        let (version, filename) = b.split_once("#").unwrap();
        ScoopCacheItem {
          entry, size, app: a.to_string(),
          filename: filename.to_string(), version: version.to_string()
        }
      })
      .collect();

    Ok(entries)
  }

  /// Collect cache files, which its name matching given `pattern`,
  /// represented as [`ScoopCacheItem`]
  pub fn get<T: AsRef<str>>(&self, pattern: T) -> Result<Vec<ScoopCacheItem>> {
    let all_cache_items = self.get_all();

    match pattern.as_ref() {
      "*" => all_cache_items,
      query => {
        if query.ends_with("*") {
          let query = query.trim_end_matches("*");
          let filtered = all_cache_items?.into_iter()
            .filter(|item| item.app.starts_with(query))
            .collect();
          Ok(filtered)
        } else {
          let filtered = all_cache_items?.into_iter()
            .filter(|item| item.app.contains(query))
            .collect();
          Ok(filtered)
        }
      }
    }
  }

  /// Remove all Scoop cache files
  pub fn clean(&self) -> Result<(), std::io::Error> {
    crate::fs::empty_dir(self.cache_dir.as_path())
  }

  /// Remove `app_name` related cache files, `*` wildcard pattern is support.
  pub fn remove<T: AsRef<str>>(&self, app_name: T) -> Result<()> {
    match app_name.as_ref() {
      "*" => self.clean()?,
      _ => {
        let cache_items = self.get(app_name.as_ref())?;
        for item in cache_items {
          std::fs::remove_file(item.entry.path())?;
        }
      }
    }

    Ok(())
  }
}
