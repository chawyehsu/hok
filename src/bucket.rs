use std::path::Path;
use std::path::PathBuf;

use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use anyhow::Result;
use crate::fs::{leaf, leaf_base, read_dir_json};

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

/// Scoop local bucket representation.
#[derive(Debug)]
pub struct Bucket {
  pub name: String,
  pub path: PathBuf,
  toplevel_manifest: bool,
}

/// ordered hash table for O(1) searching perf.
pub type Buckets = IndexMap<String, Bucket>;

#[derive(Debug)]
pub struct BucketManager {
  working_dir: PathBuf,
  buckets: IndexMap<String, Bucket>
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
  /// Create a new local bucket instance with the given path, which's parent
  /// should be `buckets`. For example:
  ///
  /// ```
  /// let path = PathBuf::from(r"C:\Scoop\buckets\my_own_bucket");
  /// let bucket = Bucket::new(path);
  /// ```
  ///
  /// The bucket name, `my_own_bucket` in above example, should only contain
  /// `a-zA-Z0-9-_` chars. Will fail if the path is invalid.
  pub fn new(path: PathBuf) -> Bucket {
    static REGEX_BUCKET_NAME: Lazy<Regex> = Lazy::new(|| {
      RegexBuilder::new(
        r".*?[\\/]buckets[\\/](?P<bucket_name>[a-zA-Z0-9-_]+)[\\/]+.*"
      ).build().unwrap()
    });
    let caps = REGEX_BUCKET_NAME.captures(path.to_str().unwrap())
      .unwrap();
    let name = caps.name("bucket_name").unwrap().as_str().to_string();
    let toplevel_manifest = !path.join("bucket").exists();

    Bucket { name, path, toplevel_manifest }
  }

  /// Return the directory [`PathBuf`] of the bucket's json manifest files.
  pub fn manifest_dir(&self) -> PathBuf {
    if !self.toplevel_manifest {
      return self.path.join("bucket");
    }

    self.path.to_path_buf()
  }

  /// Get all available apps' name in this bucket.
  pub fn available_apps(&self) -> Result<Vec<String>> {
    Ok(
      read_dir_json(&self.manifest_dir())?.into_iter().map(|path| {
        leaf_base(path.as_path())
      }).collect::<Vec<_>>()
    )
  }

  /// Get all available apps' manifest [`PathBuf`] in this bucket.
  pub fn available_manifests(&self) -> Result<Vec<PathBuf>> {
    Ok(read_dir_json(&self.manifest_dir())?)
  }

  /// Check if this bucket is a known bucket.
  pub fn is_known(&self) -> bool {
    is_known_bucket(self.name.as_ref())
  }

  /// Check if this bucket is a git repo bucket.
  pub fn is_git_repo(&self) -> bool {
    self.path.join(".git").exists()
  }
}

impl BucketManager {
  /// Create a new [`BucketManager`] from the given Scoop [`Config`]
  pub fn new(working_dir: PathBuf) -> BucketManager {
    let buckets = Self::collect_buckets(&working_dir);

    BucketManager { working_dir, buckets }
  }

  fn collect_buckets(working_dir: &Path) -> Buckets {
    let mut buckets: Buckets = IndexMap::new();

    // Ensure `buckets` dir
    crate::fs::ensure_dir(working_dir).unwrap();

    let entries = working_dir.read_dir()
      .unwrap().filter_map(Result::ok)
      .filter(|de| de.file_type().unwrap().is_dir())
      .map(|de| {
        let path = de.path();
        let name = leaf(path.as_path());
        let toplevel_manifest = !path.join("bucket").exists();
        (name.clone(), Bucket { name, path, toplevel_manifest })
      }).collect::<Vec<_>>();

    for entry in entries {
      buckets.insert(entry.0, entry.1);
    }

    buckets
  }

  /// Get all local buckets.
  pub fn get_buckets(&self) -> &Buckets {
    &self.buckets
  }

  /// Find local bucket with the given name.
  pub fn get_bucket<S: AsRef<str>>(&self, name: S) -> Option<&Bucket> {
    self.buckets.get(name.as_ref())
  }

  /// Check if the bucket with the given name is a local bucket.
  pub fn contains<S: AsRef<str>>(&self, name: S) -> bool {
    self.buckets.contains_key(name.as_ref())
  }

  #[allow(dead_code)]
  /// Update working_dir of this [`BucketManager`].
  fn update_working_dir(&mut self, working_dir: PathBuf) {
    self.working_dir = working_dir;
  }
}
