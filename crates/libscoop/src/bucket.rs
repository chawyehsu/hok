use rayon::prelude::{ParallelBridge, ParallelIterator};
use std::path::{Path, PathBuf};

use crate::{
    error::{Context, Error, Fallible},
    internal::git,
};

#[derive(Clone, Debug)]
pub struct Bucket {
    path: PathBuf,
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
    dtype: BucketDirectoryType,
}

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
    #[inline]
    pub fn from(path: &Path) -> Fallible<Bucket> {
        let path = path.to_owned();
        let name = path
            .file_name()
            .map(|n| n.to_str().unwrap().to_string())
            .unwrap();
        if !path.exists() {
            return Err(Error::BucketNotFound(name.clone()));
        }

        let mut remote_url = None;
        if path.is_dir() && path.join(".git").exists() {
            remote_url = git::remote_url_of(path.as_path(), "origin")?;
        }

        let nested_dir = path.join("bucket");
        let dtype = match nested_dir.exists() {
            false => BucketDirectoryType::V1,
            true => {
                let mut dtype = BucketDirectoryType::V2;
                let entries = nested_dir
                    .read_dir()
                    .with_context(|| format!("failed to read {}", nested_dir.display()))?;

                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.path().is_dir() {
                            dtype = BucketDirectoryType::V3;
                            break;
                        }
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
    #[inline]
    pub fn remote_url(&self) -> Option<&str> {
        self.remote_url.as_deref()
    }

    pub fn path_of_manifest(&self, name: &str) -> PathBuf {
        let filename = format!("{}.json", name);
        match self.dtype {
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
        }
    }

    /// Get manifests from the bucket.
    ///
    /// Returns a list of PathBufs of these manifest files.
    pub fn manifests(&self) -> Fallible<Vec<PathBuf>> {
        let json_files = match self.dtype {
            BucketDirectoryType::V1 => {
                let path = self.path.as_path();
                let entries = path
                    .read_dir()
                    .with_context(|| format!("failed to read dir: {}", path.display()))?
                    .par_bridge()
                    .filter_map(std::io::Result::ok)
                    .filter(|de| {
                        let path = de.path();
                        let name = path.file_name().unwrap().to_str().unwrap();
                        // Ignore npm package config file
                        path.is_file() && name.ends_with(".json") && name != "package.json"
                    })
                    .map(|de| de.path())
                    .collect::<Vec<_>>();
                entries
            }
            BucketDirectoryType::V2 => {
                let path = self.path.join("bucket");
                let entries = path
                    .read_dir()
                    .with_context(|| format!("failed to read dir: {}", path.display()))?
                    .par_bridge()
                    .filter_map(std::io::Result::ok)
                    .filter(|de| {
                        let path = de.path();
                        let name = path.file_name().unwrap().to_str().unwrap();
                        path.is_file() && name.ends_with(".json")
                    })
                    .map(|de| de.path())
                    .collect::<Vec<_>>();
                entries
            }
            BucketDirectoryType::V3 => {
                let path = self.path.join("bucket");
                let entries = path
                    .read_dir()
                    .with_context(|| format!("failed to read dir: {}", path.display()))?
                    .par_bridge()
                    .filter_map(std::io::Result::ok)
                    .filter(|de| {
                        let path = de.path();
                        let name = path.file_name().unwrap().to_str().unwrap();
                        path.is_dir() && !name.starts_with('.')
                    })
                    .flat_map(|de| {
                        let path = de.path();
                        let entries = path
                            .read_dir()
                            .with_context(|| format!("failed to read dir: {}", path.display()))
                            .unwrap()
                            .par_bridge()
                            .filter_map(std::io::Result::ok)
                            .filter(|de| {
                                let path = de.path();
                                let name = path.file_name().unwrap().to_str().unwrap();
                                path.is_file() && name.ends_with(".json")
                            })
                            .map(|de| de.path())
                            .collect::<Vec<_>>();
                        entries
                    })
                    .collect::<Vec<_>>();
                entries
            }
        };
        Ok(json_files)
    }

    /// Get manifest count of the bucket.
    #[inline]
    pub fn manifest_count(&self) -> Fallible<usize> {
        Ok(self.manifests()?.len())
    }
}
