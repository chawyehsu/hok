use anyhow::{anyhow, Result};
use std::{io::BufReader, path::PathBuf};

use serde_json::Value;

use crate::Scoop;

impl Scoop {
  pub fn manifest(&self, app_name: &str) -> Option<Value> {
    let buckets = self.get_local_buckets_name().unwrap();

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

    match serde_json::from_reader(reader) {
      Ok(m) => return Ok(m),
      Err(_e) => {
        let msg = format!("Failed to parse manifest '{}'",
          manifest_path.to_str().unwrap());
        return Err(anyhow!(msg));
      }
    }
  }

  pub fn manifest_from_url(&self, manifest_url: &str) -> Result<Value> {
    // Use proxy from Scoop's config
    let agent = match self.config["proxy"].clone() {
      Value::String(mut proxy) => {
        if !proxy.starts_with("http") {
          proxy.insert_str(0, "http://");
        }

        let proxy = ureq::Proxy::new(proxy)?;

        ureq::AgentBuilder::new()
          .proxy(proxy)
          .build()
      },
      _ => {
        ureq::AgentBuilder::new()
          .build()
      }
    };

    let body: serde_json::Value = agent.get(manifest_url)
      .call()?
      .into_json()?;

    Ok(body)
  }
}
