use std::path::PathBuf;
use std::fs::DirEntry;

use anyhow::Result;
use lazy_static::lazy_static;
use serde_json::{json, Value};
use crate::Scoop;

lazy_static! {
  static ref KNOWN_BUCKETS: Value = json!({
    "main": "https://github.com/ScoopInstaller/Main",
    "extras": "https://github.com/lukesampson/scoop-extras",
    "versions": "https://github.com/ScoopInstaller/Versions",
    "nightlies": "https://github.com/ScoopInstaller/Nightlies",
    "nirsoft": "https://github.com/kodybrown/scoop-nirsoft",
    "php": "https://github.com/ScoopInstaller/PHP",
    "nerd-fonts": "https://github.com/matthewjberger/scoop-nerd-fonts",
    "nonportable": "https://github.com/TheRandomLabs/scoop-nonportable",
    "java": "https://github.com/ScoopInstaller/Java",
    "games": "https://github.com/Calinou/scoop-games",
    "jetbrains": "https://github.com/Ash258/Scoop-JetBrains"
  });
}

impl Scoop {
  pub fn get_known_buckets() {
    let buckets = KNOWN_BUCKETS.as_object().unwrap().keys();
    for b in buckets {
      println!("{}", b);
    }
  }

  pub fn get_known_bucket_url(bucket_name: &str) -> &'static str {
    KNOWN_BUCKETS[bucket_name].as_str().unwrap()
  }

  pub fn get_added_buckets(&self) -> Result<Vec<String>> {
    let buckets = std::fs::read_dir(&self.buckets_dir)?
      .filter_map(Result::ok)
      .map(|entry| entry.file_name().to_str().unwrap().to_owned())
      .collect();
    Ok(buckets)
  }

  pub fn is_known_bucket(bucket_name: &str) -> bool {
    KNOWN_BUCKETS.as_object().unwrap().contains_key(bucket_name)
  }

  pub fn buckets(&self) {
    let buckets = self.get_added_buckets().unwrap();
    for b in buckets {
      println!("{}", b);
    }
  }

  pub fn is_added_bucket(&self, bucket_name: &str) -> bool {
    let buckets = self.get_added_buckets().unwrap();
    buckets.contains(&bucket_name.to_string())
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
