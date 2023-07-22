//! Operations that can be performed on a Scoop instance.
//!
//! # Examples
//!
//! ```rust
//! use libscoop::{Session, operation};
//! let (session, _) = Session::init().expect("failed to create session");
//! let buckets = operation::bucket_list(&session).expect("failed to get buckets");
//! println!("{} bucket(s)", buckets.len());
//! ```
use chrono::{SecondsFormat, Utc};
use futures::{executor::ThreadPool, task::SpawnExt};
use log::{debug, warn};
use std::{
    collections::HashSet,
    iter::FromIterator,
    sync::{Arc, Mutex},
};

use crate::{
    bucket::Bucket,
    cache::CacheFile,
    error::{Context, Error, Fallible},
    event::{BucketUpdateFailedCtx, Event},
    internal::{fs, git},
    package::{self, manifest::InstallInfo, Package, QueryOption},
    Session,
};

/// Add a bucket to Scoop.
pub fn bucket_add(session: &Session, name: &str, remote_url: &str) -> Fallible<()> {
    let mut path = session.get_config().root_path.clone();

    path.push("buckets");
    path.push(name);

    if path.exists() {
        return Err(Error::BucketAlreadyExists(name.to_owned()));
    }

    let config = session.get_config();
    let proxy = config.proxy();
    let remote_url = match "" != remote_url {
        true => remote_url,
        false => crate::constant::BUILTIN_BUCKET_LIST
            .iter()
            .find(|&&(n, _)| n == name)
            .map(|&(_, remote)| remote)
            .ok_or_else(|| Error::BucketAddRemoteRequired(name.to_owned()))?,
    };

    git::clone_repo(remote_url, path, proxy)
}

/// Get a list of added buckets.
pub fn bucket_list(session: &Session) -> Fallible<Vec<Bucket>> {
    let mut buckets = Vec::new();
    let buckets_dir = session.get_config().root_path.join("buckets");

    if buckets_dir.exists() {
        let entries = buckets_dir
            .read_dir()
            .with_context(|| format!("failed to read {}", buckets_dir.display()))?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().unwrap().is_dir());
        for entry in entries {
            let path = entry.path();
            match Bucket::from(&path) {
                Ok(bucket) => buckets.push(bucket),
                Err(err) => warn!("failed to parse bucket {} ({})", path.display(), err),
            }
        }
    }
    Ok(buckets)
}

/// Get a list of known (built-in) buckets.
pub fn bucket_list_known() -> Vec<(&'static str, &'static str)> {
    crate::constant::BUILTIN_BUCKET_LIST.to_vec()
}

/// Update all added buckets.
pub fn bucket_update(session: &Session) -> Fallible<()> {
    let buckets = bucket_list(session)?;
    if buckets.is_empty() {
        return Ok(());
    }

    let any_bucket_updated = Arc::new(Mutex::new(false));
    let mut tasks = Vec::new();
    let pool = ThreadPool::builder()
        .create()
        .with_context(|| "failed to create thread pool".into())?;
    let proxy = session.get_config().proxy().map(|s| s.to_owned());

    for bucket in buckets.iter() {
        let repo = bucket.path().to_owned();

        if repo.join(".git").exists() != true {
            debug!("ignored non-git bucket {}", bucket.name());
            continue;
        }

        let name = bucket.name().to_owned();
        let flag = Arc::clone(&any_bucket_updated);
        let emitter = session.emitter.clone();
        let proxy = proxy.clone();
        let task = pool
            .spawn_with_handle(async move {
                emitter
                    .send(Event::BucketUpdateStarted(name.clone()))
                    .unwrap();
                match git::reset_head(repo, proxy) {
                    Ok(_) => {
                        *flag.lock().unwrap() = true;
                        emitter.send(Event::BucketUpdateSuccessed(name)).unwrap();
                    }
                    Err(err) => {
                        let ctx: BucketUpdateFailedCtx = BucketUpdateFailedCtx {
                            name: name.clone(),
                            err_msg: err.to_string(),
                        };
                        emitter.send(Event::BucketUpdateFailed(ctx)).unwrap();
                    }
                };
            })
            .map_err(|e| Error::Custom(e.to_string()))?;
        tasks.push(task);
    }

    let joined = futures::future::join_all(tasks);
    futures::executor::block_on(joined);

    if *any_bucket_updated.lock().unwrap() {
        let time = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false);
        config_set(session, "last_update", time.as_str())?;
    }

    session.emitter.send(Event::BucketUpdateFinished).unwrap();
    Ok(())
}

/// Remove a bucket from Scoop.
pub fn bucket_remove(session: &Session, name: &str) -> Fallible<()> {
    let mut path = session.get_config().root_path.clone();
    path.push("buckets");
    path.push(name);

    if !path.exists() {
        return Err(Error::BucketNotFound(name.to_owned()));
    }

    Ok(remove_dir_all::remove_dir_all(path.as_path())
        .with_context(|| format!("failed to remove bucket {}", path.display()))?)
}

/// Get a list of downloaded cache files.
pub fn cache_list(session: &Session, query: &str) -> Fallible<Vec<CacheFile>> {
    let mut entires = session
        .get_config()
        .cache_path
        .read_dir()
        .with_context(|| {
            format!(
                "failed to read cache dir: {}",
                session.get_config().cache_path.display()
            )
        })?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().unwrap().is_file())
        .filter_map(|de| CacheFile::from(de.path()).ok())
        .collect::<Vec<_>>();
    match query {
        "" | "*" => {}
        query => {
            entires = entires
                .into_iter()
                .filter(|f| f.package_name().contains(query))
                .collect::<Vec<_>>();
        }
    }
    Ok(entires)
}

/// Remove cache files by query.
pub fn cache_remove(session: &Session, query: &str) -> Fallible<()> {
    match query {
        "*" => Ok(
            fs::empty_dir(&session.get_config().cache_path).with_context(|| {
                format!(
                    "failed to empty cache dir: {}",
                    session.get_config().cache_path.display()
                )
            })?,
        ),
        query => {
            let files = cache_list(session, query)?;
            for f in files.into_iter() {
                std::fs::remove_file(f.path()).with_context(|| {
                    format!("failed to remove cache file: {}", f.path().display())
                })?;
            }
            Ok(())
        }
    }
}

/// Get the configuation list.
pub fn config_list(session: &Session) -> Fallible<String> {
    let config = session.config.borrow();
    config.pretty()
}

/// Set a configuation key.
pub fn config_set(session: &Session, key: &str, value: &str) -> Fallible<()> {
    session.config.borrow_mut().set(key, value)
}

/// Hold or unhold a package.
pub fn package_hold(session: &Session, name: &str, flag: bool) -> Fallible<()> {
    let mut path = session.get_config().root_path.clone();
    path.push("apps");
    path.push(name);

    if !path.exists() {
        return Err(Error::PackageHoldNotInstalled(name.to_owned()));
    }

    path.push("current");
    path.push("install.json");

    if let Ok(mut install_info) = InstallInfo::parse(&path) {
        install_info.set_held(flag);
        fs::write_json(path, install_info)
    } else {
        Err(Error::PackageHoldBrokenInstall(name.to_owned()))
    }
}

// pub fn package_install(
//     session: &Session,
//     queries: Vec<&str>,
//     options: HashSet<InstallOption>,
// ) -> Fallible<()> {
//     // remove possible duplicates
//     let queries = HashSet::from_iter(queries);
//     let packages = package::resolve::resolve_packages(session, queries)?;
//     package::download::install_packages(session, packages, options)?;
//     Ok(())
// }

pub fn package_list(session: &Session, query: &str, upgradable: bool) -> Fallible<Vec<Package>> {
    let queries = HashSet::<&str>::from_iter(query.split(' ').collect::<Vec<_>>());
    let mut options = vec![];
    if upgradable {
        options.push(QueryOption::Upgradable);
    }

    package::query::query_installed(session, queries, options)
}

pub fn package_search(
    session: &Session,
    queries: Vec<&str>,
    options: Vec<QueryOption>,
) -> Fallible<Vec<Package>> {
    // remove possible duplicates
    let queries = HashSet::from_iter(queries);
    package::query::query_synced(session, queries, options)
}
