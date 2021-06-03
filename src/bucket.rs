use std::{io, path::PathBuf};
use std::fs::DirEntry;

use indexmap::IndexMap;
use once_cell::sync::Lazy;
use anyhow::Result;
use crate::config::Config;
use crate::fs;

static KNOWN_BUCKETS: Lazy<Vec<(&str, &str)>> = Lazy::new(|| {
  vec![
    ("main", "https://github.com/ScoopInstaller/Main"),
    ("extras", "https://github.com/lukesampson/scoop-extras"),
    ("versions", "https://github.com/ScoopInstaller/Versions"),
    ("nightlies", "https://github.com/ScoopInstaller/Nightlies"),
    ("nirsoft", "https://github.com/kodybrown/scoop-nirsoft"),
    ("php", "https://github.com/ScoopInstaller/PHP"),
    ("nerd-fonts", "https://github.com/matthewjberger/scoop-nerd-fonts"),
    ("nonportable", "https://github.com/TheRandomLabs/scoop-nonportable"),
    ("java", "https://github.com/ScoopInstaller/Java"),
    ("games", "https://github.com/Calinou/scoop-games"),
    ("jetbrains", "https://github.com/Ash258/Scoop-JetBrains")
  ]
});

pub struct Bucket {
  pub name: String,
  pub entry: DirEntry,
}

/// ordered hash table for O(1) searching perf.
pub type Buckets = IndexMap<String, Bucket>;

#[derive(Debug)]
pub struct BucketManager {
  bucket_dir: PathBuf
}

/// Collect known buckets
pub fn known_buckets() -> Vec<&'static str> {
  let buckets: Vec<&str> = KNOWN_BUCKETS
    .iter().map(|p| p.0).collect();
  buckets
}

/// Return url of given known `bucket_name` bucket.
pub fn known_bucket_url(bucket_name: &str) -> Option<&'static str> {
  for (name, url) in KNOWN_BUCKETS.iter() {
    if bucket_name.eq(*name) {
      return Some(url);
    }
  }

  None
}

/// Test given `bucket_name` is a known bucket or not.
pub fn is_known_bucket(bucket_name: &str) -> bool {
  known_buckets().contains(&bucket_name)
}

impl Bucket {
  pub fn new(name: String, entry: DirEntry) -> Bucket {
    Bucket { name, entry }
  }

  /// Return bucket's path
  pub fn path(&self) -> PathBuf {
    self.entry.path()
  }

  /// Return the root path that the bucket's json files are stored in.
  pub fn root(&self) -> PathBuf {
    let p = self.path();
    match p.join("bucket").exists() {
      true => p.join("bucket"),
      false => p
    }
  }

  /// Return all JSON manifest entries of the bucket
  pub fn json_entries(&self) -> Result<Vec<DirEntry>> {
    let entries = crate::fs::read_dir_json(self.root())?;
    Ok(entries)
  }

  /// Check if the bucket is a known bucket
  pub fn is_known(&self) -> bool {
    is_known_bucket(self.name.as_ref())
  }
}

impl BucketManager {
  pub fn new(config: &Config) -> BucketManager {
    let bucket_dir = PathBuf::from(
      config.get("root_path").unwrap().as_str().unwrap()
    ).join("buckets");

    BucketManager { bucket_dir }
  }

  /// Collect local buckets
  pub fn local_buckets(&self) -> Result<&Buckets, io::Error> {
    // let mut sbs = IndexMap::new(); // Can we cache the initialized map?
    static mut LOCAL_BUCKETS: Lazy<Buckets> = Lazy::new(|| {
      IndexMap::new()
    });

    unsafe {
      if LOCAL_BUCKETS.len() > 0 {
        return Ok(&LOCAL_BUCKETS)
      }
    }

    let ref buckets_dir = self.bucket_dir;
    // Ensure `buckets` dir
    crate::fs::ensure_dir(buckets_dir)?;

    let buckets: Vec<DirEntry> = std::fs::read_dir(buckets_dir)?
      .filter_map(Result::ok)
      .filter(|de| de.file_type().unwrap().is_dir())
      .collect();

    for entry in buckets {
      let name = fs::leaf_base(entry.path());

      unsafe {
        if !LOCAL_BUCKETS.contains_key(name.as_str()) {
          LOCAL_BUCKETS.insert(
            name.clone(),
            Bucket { name, entry }
          );
        }
      }
    }

    unsafe {
      Ok(&LOCAL_BUCKETS)
    }
  }

  /// Return local bucket of given `bucket_name` represented as [`ScoopBucket`]
  pub fn local_bucket<T: AsRef<str>>(&self, bucket_name: T)
    -> Result<Option<&Bucket>, io::Error> {
      Ok(self.local_buckets()?.get(bucket_name.as_ref()))
  }

  /// Test given `bucket_name` is a local bucket or not.
  pub fn is_local_bucket<T: AsRef<str>>(&self, bucket_name: T)
    -> Result<bool, io::Error> {
    Ok(self.local_buckets()?.contains_key(bucket_name.as_ref()))
  }

  /// Collect apps located in given `bucket_name` bucket.
  pub fn apps_in_local_bucket<T: AsRef<str>>(&self, bucket_name: T)
    -> Result<Vec<DirEntry>> {
    let bucket = self.local_bucket(bucket_name.as_ref())?.unwrap();
    let apps = crate::fs::read_dir_json(bucket.root())?;

    Ok(apps)
  }
}
