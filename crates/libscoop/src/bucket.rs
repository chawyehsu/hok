use rayon::prelude::{ParallelBridge, ParallelIterator};
use std::path::{Path, PathBuf};

use crate::{
    error::{Error, Fallible},
    internal::git,
};

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
    remote_url: Option<String>,

    /// The directory type of the bucket.
    dtype: BucketDirectoryType,
}

/// Bucket directory type.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum BucketDirectoryType {
    /// Bare bucket
    V1,
    /// `bucket` subdirectory
    V2,
    /// `bucket` subdirectory with nested categories
    V3,
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

        let mut remote_url = None;
        if path.is_dir() && path.join(".git").exists() {
            let check_git = git::remote_url_of(path.as_path(), "origin");

            if let Ok(some) = check_git {
                remote_url = some;
            }
        }

        let nested_dir = path.join("bucket");
        let is_nested = nested_dir.exists() && nested_dir.is_dir();
        let dtype = match is_nested {
            false => BucketDirectoryType::V1,
            true => {
                let mut dtype = BucketDirectoryType::V2;
                let entries = nested_dir.read_dir()?;

                for entry in entries.flatten() {
                    // assume it's a v3 bucket if there is any subdirectory
                    if entry.path().is_dir() {
                        dtype = BucketDirectoryType::V3;
                        break;
                    }
                }
                dtype
            }
        };
        let bucket = Bucket {
            path,
            name,
            remote_url,
            dtype,
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

    /// Get the repository url of the bucket.
    ///
    /// # Returns
    ///
    /// The remote url of the bucket, none if the bucket is not a git repository
    /// or it is a local git repository without remote url, or the git metadata
    /// is broken.
    #[inline]
    pub fn remote_url(&self) -> Option<&str> {
        self.remote_url.as_deref()
    }

    /// Get the manifest path of the given package name.
    ///
    /// # Returns
    ///
    /// The path of the manifest file, none if the package is not in the bucket.
    pub fn path_of_manifest(&self, name: &str) -> Option<PathBuf> {
        let filename = format!("{}.json", name);
        let path = match self.dtype {
            BucketDirectoryType::V1 => self.path.join(filename),
            BucketDirectoryType::V2 => {
                let mut path = self.path.join("bucket");
                path.push(filename);
                path
            }
            BucketDirectoryType::V3 => {
                let first = name.chars().take(1).last().unwrap();
                // manifest name must start with an alphanumeric character
                let category = if first.is_ascii_lowercase() {
                    first.to_string()
                } else {
                    "#".to_owned()
                };
                let mut path = self.path.join("bucket");
                path.push(category);
                path.push(filename);
                path
            }
        };

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Get manifests from the bucket.
    ///
    /// # Returns
    ///
    /// a list of PathBufs of manifest files.
    ///
    /// # Errors
    ///
    /// I/O errors will be returned if the bucket directory is not readable.
    pub fn manifests(&self) -> Fallible<Vec<PathBuf>> {
        let json_files = match self.dtype {
            BucketDirectoryType::V1 => {
                let path = self.path.as_path();
                path.read_dir()?
                    .par_bridge()
                    .filter_map(std::io::Result::ok)
                    .filter(|de| {
                        let path = de.path();
                        let name = path.file_name().unwrap().to_str().unwrap();
                        // Ignore npm package config file, that said, there will
                        // be no package named `package`, it's a reserved name.
                        path.is_file() && name.ends_with(".json") && name != "package.json"
                    })
                    .map(|de| de.path())
                    .collect::<Vec<_>>()
            }
            BucketDirectoryType::V2 => {
                let path = self.path.join("bucket");
                path.read_dir()?
                    .par_bridge()
                    .filter_map(std::io::Result::ok)
                    .filter(|de| {
                        let path = de.path();
                        let name = path.file_name().unwrap().to_str().unwrap();
                        path.is_file() && name.ends_with(".json") && name != "package.json"
                    })
                    .map(|de| de.path())
                    .collect::<Vec<_>>()
            }
            BucketDirectoryType::V3 => {
                let path = self.path.join("bucket");
                path.read_dir()?
                    .par_bridge()
                    .filter_map(std::io::Result::ok)
                    .filter(|de| {
                        let path = de.path();
                        let name = path.file_name().unwrap().to_str().unwrap();
                        path.is_dir() && !name.starts_with('.')
                    })
                    .flat_map(|de| -> Fallible<Vec<PathBuf>> {
                        let path = de.path();
                        let entries = path
                            .read_dir()?
                            .par_bridge()
                            .filter_map(std::io::Result::ok)
                            .filter(|de| {
                                let path = de.path();
                                let name = path.file_name().unwrap().to_str().unwrap();
                                path.is_file() && name.ends_with(".json") && name != "package.json"
                            })
                            .map(|de| de.path())
                            .collect::<Vec<_>>();
                        Ok(entries)
                    })
                    .flatten()
                    .collect::<Vec<_>>()
            }
        };
        Ok(json_files)
    }

    /// Get manifest count of the bucket.
    ///
    /// # Returns
    ///
    /// The count of manifests.
    ///
    /// # Errors
    ///
    /// I/O errors will be returned if the bucket directory is not readable.
    #[inline]
    pub fn manifest_count(&self) -> Fallible<usize> {
        Ok(self.manifests()?.len())
    }
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
