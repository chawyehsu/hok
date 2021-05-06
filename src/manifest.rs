use anyhow::Result;
use std::{io::BufReader, path::PathBuf};

use serde_json::Value;

use crate::Scoop;

impl Scoop {
  pub fn manifest(&self, app_name: &str) -> Option<Value> {
    let buckets = self.get_added_buckets().unwrap();

    for bucket in buckets {
      let bucket_path = self.path_of(bucket.as_str());
      let manifest_path = bucket_path.join(format!("{}.json", app_name));

      if manifest_path.exists() {
        return Some(self.manifest_from_local(manifest_path).unwrap());
      }
    }

    None
  }

  pub fn manifest_from_local(&self, manifest_path: PathBuf) -> Result<Value> {
    let file = std::fs::File::open(manifest_path.as_path())?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;
    Ok(u)
  }

  pub fn manifest_from_url(&self, manifest_url: &str) -> Result<Value> {
    let body: serde_json::Value = ureq::get(manifest_url)
      .call()?
      .into_json()?;

    Ok(body)
  }
}
