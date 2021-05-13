use anyhow::{anyhow, Result};
use std::{io::BufReader, path::{Path, PathBuf}};

use serde_json::Value;

use crate::{Scoop, fs};

pub enum ManifestFromType {
  Local(PathBuf),
  Remote(String)
}

pub struct ScoopAppManifest {
  pub app: String,
  pub bucket: Option<String>,
  pub version: String,
  pub json: Value,
  pub from: ManifestFromType,
}

impl ScoopAppManifest {
  pub fn from_path<P: AsRef<Path>>(path: P) -> Result<ScoopAppManifest> {
    let file = std::fs::File::open(path.as_ref())?;
    let reader = BufReader::new(file);

    match serde_json::from_reader(reader) {
      Ok(v) => {
        let json: Value = v;
        let version = json.get("version");
        if version.is_none() {
          let msg = format!("Failed to read version from manifest '{}'",
          path.as_ref().to_str().unwrap());
          return Err(anyhow!(msg));
        }

        return Ok(ScoopAppManifest {
          app: fs::leaf_base(path.as_ref()),
          bucket: None, // FIXME
          version: version.unwrap().to_string(),
          json,
          from: ManifestFromType::Local(path.as_ref().to_path_buf())
        });
      },
      Err(_e) => {
        let msg = format!("Failed to parse manifest '{}'",
          path.as_ref().to_str().unwrap());
        return Err(anyhow!(msg));
      }
    }
  }

  pub fn from_url<T: AsRef<str>>(url: T) -> Result<ScoopAppManifest> {
    todo!()
  }
}

impl Scoop {
  /// Find and return local manifest represented as [`ScoopAppManifest`],
  /// using given `pattern`.
  ///
  /// bucket name prefix is support, for example:
  /// ```
  /// find_local_manifest("main/gcc")
  /// ```
  pub fn find_local_manifest<T: AsRef<str>>(&self, pattern: T) -> Result<Option<ScoopAppManifest>> {
    // Detect given pattern whether having bucket name prefix
    let (bucket_name, app_name) =
      match pattern.as_ref().contains("/") {
        true => {
          let (a, b) = pattern.as_ref()
            .split_once("/").unwrap();
          (Some(a), b)
        },
        false => (None, pattern.as_ref())
      };

    match bucket_name {
      Some(bucket_name) => {
        let bucket = self.local_bucket(bucket_name)?.unwrap();
        let manifest_path = bucket.root()
          .join(format!("{}.json", app_name));
        match manifest_path.exists() {
          true => Ok(Some(ScoopAppManifest::from_path(manifest_path)?)),
          false => Ok(None)
        }
      },
      None => {
        for bucket in self.local_buckets()? {
          let manifest_path = bucket.1.root()
            .join(format!("{}.json", app_name));
          match manifest_path.exists() {
            true => return Ok(Some(ScoopAppManifest::from_path(manifest_path)?)),
            false => {}
          }
        }

        Ok(None)
      }
    }
  }

  /// Deprecated, will be replaced by ScoopAppManifest::from_url()
  #[deprecated]
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
