use std::{fs::DirEntry, path::PathBuf};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use anyhow::Result;

/// A struct represents a downloaded cache item of scoop.
#[derive(Debug)]
pub struct CacheEntry {
  entry: DirEntry,
  app_name: String,
  version: String,
  file_name: String,
}

#[derive(Debug)]
pub struct CacheManager {
  working_dir: PathBuf,
}

impl CacheEntry {
  /// Create a new Scoop [`CacheEntry`] from given [`DirEntry`]
  ///
  /// Caveat: the constructor does not validate given DirEntry.
  pub fn new(entry: DirEntry) -> CacheEntry {
    let fname = entry.file_name().into_string().unwrap();
    let meta = fname.split("#").collect::<Vec<_>>();
    let (app_name, version, file_name) = (
      meta[0].to_string(), meta[1].to_string(), meta[2].to_string()
    );

    CacheEntry { entry, app_name, version, file_name }
  }

  pub fn app_name(&self) -> &str {
    &self.app_name
  }

  pub fn file_name(&self) -> &str {
    &self.file_name
  }

  pub fn size(&self) -> u64 {
    self.entry.metadata().unwrap().len()
  }

  pub fn version(&self) -> &str {
    &self.version
  }
}

impl CacheManager {
  pub fn new(working_dir: PathBuf) -> CacheManager {
    CacheManager { working_dir }
  }

  /// Collect all cache files represented as [`CacheEntry`]
  pub fn get_all(&self) -> Result<Vec<CacheEntry>> {
    static RE: Lazy<Regex> = Lazy::new(|| {
      RegexBuilder::new(r"(?P<app>[a-zA-Z0-9-_.]+)#(?P<version>[a-zA-Z0-9-.]+)#(?P<url>.*)")
      .build().unwrap()
    });

    let entries = self.working_dir.read_dir()?
      .filter_map(Result::ok)
      .filter(|de| RE.is_match(de.file_name().to_str().unwrap()))
      .map(|entry| CacheEntry::new(entry))
      .collect();

    Ok(entries)
  }

  /// Collect cache files, which its name matching given `pattern`,
  /// represented as [`CacheEntry`]
  pub fn get<T: AsRef<str>>(&self, pattern: T) -> Result<Vec<CacheEntry>> {
    let all_cache_items = self.get_all();

    match pattern.as_ref() {
      "*" => all_cache_items,
      mut query => {
        if query.ends_with("*") {
          query = query.trim_end_matches("*")
        }

        let filtered = all_cache_items?.into_iter()
            .filter(|ce| ce.app_name().starts_with(query))
            .collect();
          Ok(filtered)
      }
    }
  }

  /// Remove all Scoop cache files
  pub fn clean(&self) -> Result<(), std::io::Error> {
    crate::fs::empty_dir(&self.working_dir)
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
