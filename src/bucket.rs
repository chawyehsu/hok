use std::io;
use std::fs::DirEntry;

use once_cell::sync::Lazy;
use anyhow::Result;
use crate::Scoop;

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

pub struct ScoopBucket {
  pub name: String,
  pub entry: Option<DirEntry>,
  pub remote: Option<String>
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


impl Scoop {
  /// Collect local buckets
  pub fn local_buckets(&self) -> Result<Vec<ScoopBucket>, io::Error> {
    let ref buckets_dir = self.buckets_dir;
    // Ensure `buckets` dir
    crate::fs::ensure_dir(buckets_dir)?;

    let mut sbs = Vec::new();
    let buckets: Vec<DirEntry> = std::fs::read_dir(buckets_dir)?
      .filter_map(Result::ok)
      .filter(|de| de.file_type().unwrap().is_dir())
      .collect();

    for b in buckets {
      let name = b.file_name().into_string().unwrap();
      sbs.push(ScoopBucket {
        name,
        entry: Some(b),
        remote: None // FIXME
      });
    }

    Ok(sbs)
  }

  /// Return [`DirEntry`] of given local `bucket_name` bucket.
  pub fn local_bucket_entry<T: AsRef<str>>(&self, bucket_name: T)
    -> Result<Option<DirEntry>, io::Error> {
    for bucket in self.local_buckets()? {
      if bucket.name.eq(bucket_name.as_ref()) {
        return Ok(bucket.entry);
      }
    }

    Ok(None)
  }

  /// Test given `bucket_name` is a local bucket or not.
  pub fn is_local_bucket<T: AsRef<str>>(&self, bucket_name: T)
    -> Result<bool, io::Error> {
    let buckets: Vec<String> = self.local_buckets()?
      .iter().map(|p| p.name.to_string()).collect();

    Ok(buckets.contains(&bucket_name.as_ref().to_string()))
  }

  /// Collect apps located in given `bucket_name` bucket.
  pub fn apps_in_local_bucket<T: AsRef<str>>(&self, bucket_name: T)
    -> Result<Vec<DirEntry>> {
    let entry = self.local_bucket_entry(bucket_name.as_ref())?.unwrap();
    let apps = crate::fs::read_dir_json(entry.path())?;

    Ok(apps)
  }
}
