use chrono::{SecondsFormat, Utc};
use futures::{executor::ThreadPool, task::SpawnExt};
use log::warn;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::{Context, Error, Fallible};
use crate::util::git::Git;
use crate::Session;

pub type BucketList = Vec<Bucket>;

#[derive(Clone, Debug)]
pub struct Bucket {
    path: PathBuf,
    name: String,
    repository: String,
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

#[derive(Clone, Debug)]
pub struct BucketUpdateContext {
    pub name: String,
    pub state: BucketUpdateState,
}

#[derive(Clone, Debug)]
pub enum BucketUpdateState {
    Started,
    Failed(String),
    Successed,
}

impl Bucket {
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
        let repository = crate::util::git::git_remote_of(path.as_path())?;

        let dir = path.join("bucket");
        let structure = match dir.exists() {
            false => BucketDirectoryType::V1,
            true => {
                let mut structure = BucketDirectoryType::V2;
                let entries = dir
                    .read_dir()
                    .with_context(|| format!("failed to read dir: {}", dir.display()))?;

                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.path().is_dir() {
                            structure = BucketDirectoryType::V3;
                            break;
                        }
                    }
                }
                structure
            }
        };
        let bucket = Bucket {
            path,
            name,
            repository,
            dtype: structure,
        };

        Ok(bucket)
    }

    /// Get the name of the bucket.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the repository url of the bucket.
    #[inline]
    pub fn repository(&self) -> &str {
        &self.repository
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
                        // Only files, and avoid npm package config file
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
                        // Only files
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
                        // Only directories
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
                                // Only files
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

    /// Get the package count of the bucket.
    #[inline]
    pub fn package_count(&self) -> Fallible<usize> {
        Ok(self.manifests()?.len())
    }
}

impl BucketUpdateContext {
    #[inline]
    pub fn new(name: String) -> Self {
        Self {
            name,
            state: BucketUpdateState::Started,
        }
    }

    #[inline]
    pub fn failed(&mut self, err: String) {
        self.state = BucketUpdateState::Failed(err);
    }

    #[inline]
    pub fn successed(&mut self) {
        self.state = BucketUpdateState::Successed;
    }
}

pub fn bucket_add(session: &Session, name: &str, repo: &str) -> Fallible<()> {
    let local_path = session.config.root_path.join("buckets").join(name);
    if local_path.exists() {
        return Err(Error::BucketAlreadyExists(name.to_owned()));
    }

    let remote_url = match "" != repo {
        true => repo,
        false => crate::constants::BUILTIN_BUCKET_LIST
            .iter()
            .find(|(n, _)| n == &name)
            .map(|&(_, repo)| repo)
            .ok_or_else(|| Error::BucketMissingRepo(name.to_owned()))?,
    };

    Git::new(session).clone_repo(local_path, remote_url)
}

pub fn bucket_list(session: &Session) -> Fallible<BucketList> {
    let mut buckets = BucketList::new();
    let buckets_dir = session.config.root_path.join("buckets");

    if buckets_dir.exists() {
        let entries = buckets_dir
            .read_dir()
            .with_context(|| format!("failed to read buckets dir: {}", buckets_dir.display()))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().unwrap().is_dir());
        for entry in entries {
            match Bucket::from(&entry.path()) {
                Ok(bucket) => buckets.push(bucket),
                Err(err) => warn!("failed to load bucket: {}", err),
            }
        }
    }
    Ok(buckets)
}

pub fn bucket_remove(session: &Session, name: &str) -> Fallible<()> {
    let path = session.config.root_path.join("buckets").join(name);
    if !path.exists() {
        return Err(Error::BucketNotFound(name.to_owned()));
    }

    Ok(remove_dir_all::remove_dir_all(path.as_path())
        .with_context(|| format!("failed to remove bucket: {}", path.display()))?)
}

pub fn bucket_update<F>(session: &mut Session, mut callback: F) -> Fallible<()>
where
    F: FnMut(BucketUpdateContext) + Send + 'static,
{
    let buckets = bucket_list(session)?;
    if buckets.is_empty() {
        return Ok(());
    }

    let mut tasks = Vec::new();
    let git = Arc::new(Git::new(session));
    let pool = ThreadPool::builder()
        .create()
        .with_context(|| "failed to create thread pool".into())?;
    let should_update_time = Arc::new(Mutex::new(false));

    let (tx, rx) = std::sync::mpsc::channel();

    let report = pool
        .spawn_with_handle(async move {
            while let Ok(ctx) = rx.recv() {
                callback(ctx);
            }
        })
        .map_err(|e| Error::Custom(e.to_string()))?;
    tasks.push(report);

    for bucket in buckets.iter() {
        let repo = session.config.root_path.join("buckets").join(bucket.name());
        let name = bucket.name().to_owned();

        let git = Arc::clone(&git);
        let should_update_time = Arc::clone(&should_update_time);

        let tx = tx.clone();
        let task = pool
            .spawn_with_handle(async move {
                let mut bctx = BucketUpdateContext::new(name);
                // emit
                tx.send(bctx.clone()).unwrap();
                // do git update
                match git.reset_head(repo) {
                    Ok(_) => {
                        bctx.successed();
                        *should_update_time.lock().unwrap() = true;
                    }
                    Err(e) => bctx.failed(e.to_string()),
                }
                // emit
                tx.send(bctx).unwrap();
            })
            .map_err(|e| Error::Custom(e.to_string()))?;
        tasks.push(task);
    }
    drop(tx);

    let joined = futures::future::join_all(tasks);
    futures::executor::block_on(joined);

    if *should_update_time.lock().unwrap() {
        // Update `lastupdate`
        let time = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false);
        session.config_set("last_update", time.as_str())?;
    }

    Ok(())
}
