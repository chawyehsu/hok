use crate::{util::walk_dir_json, Config, Git, ScoopResult};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use super::AvailableApp;

/// Scoop local bucket representation.
#[derive(Debug)]
pub struct Bucket<'cfg> {
    config: &'cfg Config,
    name: String,
}

impl<'cfg> Bucket<'cfg> {
    /// Create a [`Bucket`] representation.
    #[inline]
    pub(crate) fn new<S: AsRef<str>>(config: &Config, name: S) -> Bucket {
        Bucket {
            config,
            name: name.as_ref().to_owned(),
        }
    }

    /// Return `name` of this bucket.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Return absolute `path` of this bucket.
    #[inline]
    pub fn path(&self) -> PathBuf {
        self.config.buckets_path().join(self.name())
    }

    /// Return the root path that contains manifest files. There are two types
    /// of bucket directory structures supported by Scoop.
    ///
    /// One is the older format which stores all manifest files in the top-level
    /// folder of the bucket. Another one is the newer format containing a sub
    /// folder called `bucket`, and all manifest files are stored in this sub
    /// folder.
    ///
    /// This method returns `self.path.join("bucket")` when the bucket's type
    /// is the newer one, otherwise returns a `self.path` copy.
    #[inline]
    pub fn manifest_dir(&self) -> PathBuf {
        let inner = self.path().join("bucket");
        match inner.exists() {
            true => inner,
            false => self.path(),
        }
    }

    /// Return manfiest `path` of the given `name`d app in this bucket.
    #[inline]
    pub(crate) fn manifest_of(&self, name: &str) -> PathBuf {
        self.manifest_dir().join(format!("{}.json", name))
    }

    /// Check if this bucket contains app named `name`.
    #[inline]
    pub fn contains_app(&self, name: &str) -> bool {
        self.manifest_of(name).exists()
    }

    /// Get the app named `name` in this bucket.
    ///
    /// ## Errors
    ///
    /// If the process failed to read manifest file, then the function will
    /// bubble up an [`std::io::error::Error`].
    pub fn app(&self, name: &str) -> ScoopResult<AvailableApp<'cfg>> {
        let bucket = self.name.as_str();
        if !self.contains_app(name) {
            anyhow::bail!("'{}' bucket doesn't have app '{}'", bucket, name);
        }
        let config = self.config;
        let path = self.manifest_of(name);
        Ok(AvailableApp::new(config, path)?)
    }

    /// Get all available apps in this bucket.
    ///
    /// ## Errors
    ///
    /// If the process failed to read manifest JSON files, then the function
    /// will bubble up an [`std::io::error::Error`].
    ///
    /// It returns a `serde_json::Error` when the JSON deserialization fails.
    pub fn apps(&self) -> ScoopResult<Vec<AvailableApp<'cfg>>> {
        let config = self.config;
        let apps = Arc::new(Mutex::new(Vec::new()));
        let json_files = walk_dir_json(&self.manifest_dir())?;
        json_files.into_par_iter().for_each(|path| {
            let app = AvailableApp::new(config, path).unwrap();
            apps.lock().unwrap().push(app);
        });
        let res = Arc::try_unwrap(apps).unwrap().into_inner().unwrap();
        Ok(res)
    }

    /// Update this bucket.
    #[inline]
    pub fn update(&self) -> ScoopResult<()> {
        Git::new(self.config).reset_head(self.path())
    }
}
