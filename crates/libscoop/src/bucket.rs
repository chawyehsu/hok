use once_cell::sync::OnceCell;
use rayon::prelude::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use crate::error::{Error, Fallible};
use crate::internal;
use crate::Session;

/// Scoop bucket representation.
///
/// A fact about Scoop bucket is that it is just a folder containing package
/// manifest files in JSON format. It could be simply a local directory or a
/// git repository.
#[derive(Clone, Debug)]
pub struct Bucket {
    /// The local path of the bucket.
    path: PathBuf,

    /// The name of the bucket.
    name: String,

    ///  The remote subscription url of the bucket.
    ///
    /// A Scoop bucket is generally a subscription to a remote git repository,
    /// this field will be the remote url of the git repository unless the git
    /// metadata is broken.
    ///
    /// Non-git bucket is also supported by Scoop, mostly it is a local directory
    /// which does not have a remote url, and bucket update is not supported.
    remote_url: OnceCell<Option<String>>,
}

impl Bucket {
    /// Create a bucket representation from a given local path.
    ///
    /// # Returns
    ///
    /// A bucket representation.
    ///
    /// # Errors
    ///
    /// This method will return an error if the given path does not exist. I/O
    /// errors will be returned if the bucket directory is not readable.
    pub fn from(path: &Path) -> Fallible<Bucket> {
        let path = path.to_owned();
        let name = path
            .file_name()
            .map(|n| n.to_str().unwrap().to_string())
            .unwrap();

        if !path.exists() {
            return Err(Error::BucketNotFound(name));
        }

        let bucket = Bucket {
            path,
            name,
            remote_url: OnceCell::new(),
        };

        Ok(bucket)
    }

    /// Get the name of the bucket.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the path of the bucket.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get manifest count of the bucket.
    ///
    /// # Returns
    ///
    /// The count of manifests in the bucket.
    #[inline]
    pub fn manifest_count(&self) -> usize {
        self.manifests().map(|v| v.len()).unwrap_or(0)
    }

    /// Get the remote url of the bucket.
    ///
    /// # Returns
    ///
    /// The git remote url of the bucket, otherwise none if the bucket is not a
    /// git repository, or it's a local git repository not having a remote url
    /// or the git metadata is broken.
    pub fn remote_url(&self) -> Option<&str> {
        self.remote_url
            .get_or_init(|| internal::git::remote_url_of(self.path(), "origin").unwrap_or(None))
            .as_deref()
    }

    /// Get the source path of the bucket.
    ///
    /// # Returns
    ///
    /// Either the remote url of the bucket or the local path of the bucket.
    #[inline]
    pub fn source(&self) -> &str {
        self.remote_url().unwrap_or(self.path().to_str().unwrap())
    }

    /// Get the manifest path of the given package name.
    ///
    /// # Returns
    ///
    /// The path of the manifest file, none if the package is not in the bucket.
    pub(crate) fn path_of_manifest(&self, name: &str) -> Option<PathBuf> {
        let filename = format!("{}.json", name);

        let mut path = self.path().to_path_buf();
        path.push(&filename);

        if path.exists() {
            return Some(path);
        } else {
            path.pop();
            path.push("bucket");
            path.push(&filename);

            if path.exists() {
                return Some(path);
            } else {
                let first = name.chars().take(1).last().unwrap();
                let category = if first.is_ascii_lowercase() {
                    first.to_string()
                } else {
                    "#".to_owned()
                };

                path.pop();
                path.push(&category);
                path.push(&filename);

                if path.exists() {
                    return Some(path);
                }
            }
        }

        None
    }

    /// Get manifests from the bucket.
    ///
    /// # Returns
    ///
    /// a list of DirEntry of manifest files.
    ///
    /// # Errors
    ///
    /// I/O errors will be returned if the bucket directory is not readable.
    pub(crate) fn manifests(&self) -> Fallible<Vec<DirEntry>> {
        let mut path = self.path().to_owned();
        path.push("bucket");

        let iter = if let Ok(entries) = par_read_dir(&path) {
            let (dirs, files): (Vec<DirEntry>, Vec<DirEntry>) =
                entries.partition(|de| de.file_type().unwrap().is_dir());

            // If the inner `bucket` directory contains subdirectories then it
            // is considered as a bucket with categories and we need to search
            // all subdirectories for manifest files.
            //
            // Category support was introduced in Scoop v0.3.0:
            // https://github.com/ScoopInstaller/Scoop/pull/5119
            if dirs.is_empty() {
                files.into_par_iter()
            } else {
                dirs.into_par_iter()
                    .filter_map(|de| par_read_dir(&de.path()).ok())
                    .flatten()
                    .collect::<Vec<_>>()
                    .into_par_iter()
            }
        } else {
            path.pop();
            par_read_dir(&path)?.collect::<Vec<_>>().into_par_iter()
        };

        let ret = iter.filter(is_manifest).collect::<Vec<_>>();

        Ok(ret)
    }
}

/// Helper function to interate entries in a directory in parallel.
fn par_read_dir(path: &Path) -> std::io::Result<impl ParallelIterator<Item = DirEntry>> {
    Ok(path.read_dir()?.par_bridge().filter_map(|de| de.ok()))
}

/// Helper function to check if a directory entry is a manifest file.
fn is_manifest(dir_entry: &DirEntry) -> bool {
    let filename = dir_entry.file_name();
    let name = filename.to_str().unwrap();
    let is_file = dir_entry.file_type().unwrap().is_file();
    // Ignore npm package config file, that said, there will
    // be no package named `package`, it's a reserved name.
    is_file && name.ends_with(".json") && name != "package.json"
}

/// Get a list of added buckets.
///
/// # Note
///
/// The returned list are unsorted.
pub fn bucket_added(session: &Session) -> Fallible<Vec<Bucket>> {
    let mut buckets = vec![];
    let buckets_dir = session.config().root_path().join("buckets");

    match buckets_dir.read_dir() {
        Err(err) => {
            warn!("failed to read buckets dir (err: {})", err);
        }
        Ok(entries) => {
            buckets = entries
                .par_bridge()
                .filter_map(|entry| {
                    if let Ok(entry) = entry {
                        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        let path = entry.path();

                        if is_dir {
                            match Bucket::from(&path) {
                                Err(err) => {
                                    warn!(
                                        "failed to parse bucket {} (err: {})",
                                        path.display(),
                                        err
                                    )
                                }
                                Ok(bucket) => return Some(bucket),
                            }
                        }
                    }
                    None
                })
                .collect::<Vec<_>>();
        }
    };

    Ok(buckets)
}

/// Bucket update progress context.
#[derive(Clone)]
pub struct BucketUpdateProgressContext {
    /// The name of the bucket.
    name: String,

    /// The update progress state of the bucket.
    state: BucketUpdateState,
}

impl BucketUpdateProgressContext {
    pub fn new(name: &str) -> BucketUpdateProgressContext {
        BucketUpdateProgressContext {
            name: name.to_owned(),
            state: BucketUpdateState::Started,
        }
    }

    /// Get the name of the bucket associated with this context.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the update progress state of the bucket.
    pub fn state(&self) -> &BucketUpdateState {
        &self.state
    }

    pub(crate) fn set_succeeded(&mut self) {
        self.state = BucketUpdateState::Succeeded;
    }

    pub(crate) fn set_failed(&mut self, msg: &str) {
        self.state = BucketUpdateState::Failed(msg.to_owned());
    }
}

/// Bucket update progress state.
#[derive(Clone, PartialEq)]
pub enum BucketUpdateState {
    /// The bucket is started to update.
    Started,

    /// The bucket is failed to update with the given error message.
    Failed(String),

    /// The bucket is updated successfully.
    Succeeded,
}

impl BucketUpdateState {
    /// Check if the state is started.
    pub fn started(&self) -> bool {
        self == &BucketUpdateState::Started
    }

    /// Check if the state is succeeded.
    pub fn succeeded(&self) -> bool {
        self == &BucketUpdateState::Succeeded
    }

    /// Check if the state is failed.
    pub fn failed(&self) -> Option<&str> {
        match self {
            BucketUpdateState::Failed(msg) => Some(msg),
            _ => None,
        }
    }
}
