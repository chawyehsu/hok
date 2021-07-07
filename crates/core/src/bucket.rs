use crate::{
    error::ScoopResult,
    fs::{leaf, leaf_base, walk_dir_json},
    Config, Git,
};
use indexmap::IndexMap;
use lazycell::LazyCell;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::result::Result;

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

/// Scoop local bucket representation.
#[derive(Debug)]
pub struct Bucket<'a> {
    config: &'a Config,
    name: String,
    path: PathBuf,
}

impl<'a> Bucket<'a> {
    /// Return the root path of this bucket's manifest files.
    ///
    /// It returns `self.path.join("bucket")` when this bucket contains a sub
    /// folder called `bucket`, or just returns a `self.path` copy.
    #[inline]
    pub fn manifest_root(&self) -> PathBuf {
        let inner = self.path.join("bucket");
        match inner.exists() {
            true => inner,
            false => self.path.clone(),
        }
    }

    /// Get all available apps' name in this bucket.
    #[inline]
    pub fn apps(&self) -> ScoopResult<Vec<String>> {
        Ok(self
            .manifests()?
            .into_iter()
            .map(|p| leaf_base(p.as_path()))
            .collect())
    }

    /// Get all available apps' manifest [`PathBuf`] in this bucket.
    #[inline]
    pub fn manifests(&self) -> ScoopResult<Vec<PathBuf>> {
        Ok(walk_dir_json(&self.manifest_root())?)
    }

    /// Update this bucket.
    #[inline]
    pub fn update(&self) -> ScoopResult<()> {
        Git::new(self.config).reset_head(self.path.as_path())
    }
}

/// A representation of a list of buckets, having an O(1) searching perf.
#[derive(Debug)]
pub struct Buckets<'a>(IndexMap<String, Bucket<'a>>);

impl<'a> std::ops::Deref for Buckets<'a> {
    type Target = IndexMap<String, Bucket<'a>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The manager of Scoop buckets.
#[derive(Debug)]
pub struct BucketManager<'a> {
    config: &'a Config,
    working_dir: PathBuf,
    buckets: LazyCell<Buckets<'a>>,
}

impl<'a> BucketManager<'a> {
    /// Create a new [`BucketManager`] with the given Scoop [`Config`].
    pub fn new(config: &'a Config) -> BucketManager<'a> {
        let working_dir = config.get_root_path().join("buckets");
        let buckets = LazyCell::new();
        BucketManager {
            config,
            working_dir,
            buckets,
        }
    }

    /// Add a new local bucket with the given bucket `name` and an optional
    /// `repo` url. The `repo` url is required when the given bucket `name`
    /// is not a known bucket.
    pub fn add_bucket<S>(&self, name: S, repo: Option<S>) -> ScoopResult<()>
    where
        S: AsRef<str>,
    {
        let bucket_name = name.as_ref();
        if self.has(bucket_name) {
            anyhow::bail!("bucket '{}' already exists", bucket_name);
        }

        if repo.is_none() && !self.is_known(bucket_name) {
            anyhow::bail!(
                "'{}' is not a known bucket, <repo> is required",
                bucket_name
            );
        }

        let git = Git::new(self.config);
        let local_path = self.working_dir.join(bucket_name);
        let remote_url = match repo {
            Some(repo) => repo.as_ref().to_owned(),
            None => self.known_bucket_url(bucket_name).to_owned(),
        };

        git.clone(local_path, remote_url)
    }

    /// Remove a local bucket with the given bucket `name`.
    pub fn remove_bucket<S>(&self, name: S) -> ScoopResult<()>
    where
        S: AsRef<str>,
    {
        let bucket_name = name.as_ref();
        if !self.has(bucket_name) {
            anyhow::bail!("bucket '{}' does not exist", bucket_name);
        }

        let bucket_path = self.working_dir.join(bucket_name);
        Ok(remove_dir_all::remove_dir_all(bucket_path)?)
    }

    /// Get all local buckets.
    pub fn buckets(&self) -> &Buckets {
        if self.buckets.filled() {
            return self.buckets.borrow().unwrap();
        }

        if self.working_dir.exists() {
            let mut buckets = IndexMap::new();
            self.working_dir
                .read_dir()
                .unwrap()
                .filter_map(Result::ok)
                .filter(|de| de.file_type().unwrap().is_dir())
                .for_each(|de| {
                    let path = de.path();
                    let name = leaf(path.as_path());
                    let bucket = Bucket {
                        config: self.config,
                        name: name.clone(),
                        path,
                    };
                    buckets.insert(name, bucket);
                });

            drop(self.buckets.fill(Buckets(buckets)));
        } else {
            drop(self.buckets.fill(Buckets(IndexMap::new())));
        }

        self.buckets.borrow().unwrap()
    }

    /// Get local bucket with the given bucket name.
    #[inline]
    pub fn bucket<S: AsRef<str>>(&self, name: S) -> Option<&Bucket> {
        self.buckets().get(name.as_ref())
    }

    /// Get all known buckets' name.
    pub fn known_buckets(&self) -> Vec<&'static str> {
        KNOWN_BUCKETS.keys().map(|k| *k).collect::<Vec<_>>()
    }

    /// Check if the bucket of the given `name` is a known bucket.
    /// Computes in O(1) time (average).
    fn is_known<S: AsRef<str>>(&self, bucket_name: S) -> bool {
        KNOWN_BUCKETS.contains_key(bucket_name.as_ref())
    }

    /// Check if the given `bucket_name` is a known bucket.
    /// Computes in O(1) time (average).
    fn known_bucket_url(&self, bucket_name: &str) -> &str {
        *(KNOWN_BUCKETS.get(bucket_name).unwrap())
    }

    /// Check if the bucket of the given `name` already exists.
    #[inline]
    fn has<S: AsRef<str>>(&self, name: S) -> bool {
        self.working_dir.join(name.as_ref()).exists()
    }
}
