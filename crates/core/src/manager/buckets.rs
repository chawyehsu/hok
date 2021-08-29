use crate::model::Bucket;
use crate::util::leaf;
use crate::Config;
use crate::Git;
use crate::ScoopResult;

use indexmap::IndexMap;
use once_cell::sync::Lazy;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::result::Result;
use std::sync::{Arc, Mutex};

static KNOWN_BUCKETS: Lazy<IndexMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut known = IndexMap::new();
    vec![
        ("main", "https://github.com/ScoopInstaller/Main"),
        ("extras", "https://github.com/lukesampson/scoop-extras"),
        ("versions", "https://github.com/ScoopInstaller/Versions"),
        ("nightlies", "https://github.com/ScoopInstaller/Nightlies"),
        ("nirsoft", "https://github.com/kodybrown/scoop-nirsoft"),
        ("php", "https://github.com/ScoopInstaller/PHP"),
        (
            "nerd-fonts",
            "https://github.com/matthewjberger/scoop-nerd-fonts",
        ),
        (
            "nonportable",
            "https://github.com/TheRandomLabs/scoop-nonportable",
        ),
        ("java", "https://github.com/ScoopInstaller/Java"),
        ("games", "https://github.com/Calinou/scoop-games"),
        ("jetbrains", "https://github.com/Ash258/Scoop-JetBrains"),
    ]
    .into_iter()
    .for_each(|(k, v)| {
        known.insert(k, v);
    });

    known
});

/// The manager of Scoop buckets.
#[derive(Debug)]
pub struct BucketManager<'cfg> {
    config: &'cfg Config,
}

impl<'cfg> BucketManager<'cfg> {
    /// Create a new [`BucketManager`] with the given Scoop [`Config`].
    pub fn new(config: &Config) -> BucketManager {
        BucketManager { config }
    }

    /// Add a new local bucket with the given bucket `name` and an optional
    /// `repo` url. The `repo` url is required when the given bucket `name`
    /// is not a known bucket.
    pub fn add_bucket<S>(&self, name: S, repo: Option<S>) -> ScoopResult<()>
    where
        S: AsRef<str>,
    {
        let bucket_name = name.as_ref();
        if self.contains(bucket_name) {
            anyhow::bail!("bucket '{}' already exists", bucket_name);
        }
        if repo.is_none() && !self.is_known(bucket_name) {
            anyhow::bail!(
                "'{}' is not a known bucket, <repo> is required",
                bucket_name
            );
        }
        let git = Git::new(self.config);
        let local_path = self.config.buckets_path().join(bucket_name);
        let remote_url = match repo {
            Some(repo) => repo.as_ref().to_owned(),
            None => self.known_bucket_url(bucket_name).to_owned(),
        };
        git.clone(local_path, remote_url)
    }

    /// Remove a local bucket with the given bucket `name`.
    pub fn remove_bucket<S: AsRef<str>>(&self, name: S) -> ScoopResult<()> {
        let bucket_name = name.as_ref();
        if !self.contains(bucket_name) {
            anyhow::bail!("bucket '{}' does not exist", bucket_name);
        }
        let bucket_path = self.config.buckets_path().join(bucket_name);
        Ok(remove_dir_all::remove_dir_all(bucket_path)?)
    }

    /// Get all local buckets.
    pub fn buckets(&self) -> Vec<Bucket<'cfg>> {
        let buckets_dir = self.config.buckets_path();
        let config = self.config;
        if buckets_dir.exists() {
            let buckets = Arc::new(Mutex::new(Vec::new()));
            buckets_dir
                .read_dir()
                .unwrap()
                .par_bridge()
                .filter_map(Result::ok)
                .filter(|de| de.file_type().unwrap().is_dir())
                .for_each(|de| {
                    let name = leaf(de.path().as_path());
                    let bucket = Bucket::new(config, name);
                    buckets.lock().unwrap().push(bucket);
                });
            if buckets.lock().unwrap().len() > 0 {
                buckets.lock().unwrap().sort_by_key(|b| b.name().to_owned());
                return Arc::try_unwrap(buckets).unwrap().into_inner().unwrap();
            }
        }
        vec![]
    }

    /// Return local bucket with the given bucket name. If the given bucket does
    /// no exist, returns `None`.
    #[inline]
    pub fn bucket<S: AsRef<str>>(&self, name: S) -> Option<Bucket<'cfg>> {
        let name = name.as_ref();
        match self.config.buckets_path().join(name).exists() {
            true => Some(Bucket::new(self.config, name)),
            false => None,
        }
    }

    /// Get all known buckets' name.
    #[inline]
    pub fn known_buckets(&self) -> Vec<&'static str> {
        KNOWN_BUCKETS.keys().map(|k| *k).collect::<Vec<_>>()
    }

    /// Check if the bucket of the given `name` is a known bucket.
    /// Computes in O(1) time (average).
    #[inline]
    fn is_known<S: AsRef<str>>(&self, bucket_name: S) -> bool {
        KNOWN_BUCKETS.contains_key(bucket_name.as_ref())
    }

    /// Check if the given `bucket_name` is a known bucket.
    /// Computes in O(1) time (average).
    #[inline]
    fn known_bucket_url(&self, bucket_name: &str) -> &str {
        *(KNOWN_BUCKETS.get(bucket_name).unwrap())
    }

    /// Check if the bucket of the given `bucket_name` already exists.
    #[inline]
    fn contains<S: AsRef<str>>(&self, bucket_name: S) -> bool {
        self.config
            .buckets_path()
            .join(bucket_name.as_ref())
            .exists()
    }
}
