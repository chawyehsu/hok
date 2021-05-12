use std::{collections::HashMap, path::PathBuf};
use std::fs::DirEntry;

use once_cell::sync::Lazy;
use anyhow::Result;
use crate::Scoop;

static KNOWN_BUCKETS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
  let knowns = vec![
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
  ];

  let mut m = HashMap::new();
  for (bucket, url) in knowns.iter() {
    m.insert(*bucket, *url);
  }
  return m;
});

impl Scoop {
  pub fn get_local_buckets_entry(&self) -> Result<Vec<DirEntry>> {
    let buckets = std::fs::read_dir(&self.buckets_dir)?
      .filter_map(Result::ok)
      .collect();
    Ok(buckets)
  }

  pub fn get_known_buckets() {
    let buckets: Vec<&str> = KNOWN_BUCKETS.iter().map(|p| *p.0).collect();
    for b in buckets {
      println!("{}", b);
    }
  }

  pub fn get_known_bucket_url(bucket_name: &str) -> &'static str {
    KNOWN_BUCKETS.get(bucket_name).unwrap()
  }

  pub fn get_local_buckets_name(&self) -> Result<Vec<String>> {
    let buckets = std::fs::read_dir(&self.buckets_dir)?
      .filter_map(Result::ok)
      .filter_map(|x| x.file_name().into_string().ok())
      .collect();
    Ok(buckets)
  }

  pub fn is_known_bucket(bucket_name: &str) -> bool {
    KNOWN_BUCKETS.contains_key(bucket_name)
  }

  pub fn buckets(&self) {
    let buckets = self.get_local_buckets_name().unwrap();
    for b in buckets {
      println!("{}", b);
    }
  }

  pub fn is_added_bucket(&self, bucket: &str) -> bool {
    let buckets = self.get_local_buckets_name().unwrap();
    buckets.contains(&bucket.to_string())
  }

  pub fn path_of(&self, bucket_name: &str) -> PathBuf {
    let p = self.buckets_dir.join(bucket_name);

    if p.join("bucket").exists() {
      p.join("bucket")
    } else {
      p
    }
  }

  pub fn apps_in_bucket(&self, bucket_name: &str) -> Result<Option<Vec<DirEntry>>> {
    let p = self.path_of(bucket_name);

    let entries: Vec<DirEntry> = std::fs::read_dir(p.as_path())?
      .filter_map(Result::ok)
      .filter(|entry| {
        let fname = entry.file_name();
        fname.to_str().unwrap().ends_with(".json")
      })
      .collect();

    if entries.len() == 0 {
      Ok(None)
    } else {
      Ok(Some(entries))
    }
  }
}
