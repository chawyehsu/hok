#![allow(unused)]
//! Operations that can be performed on a Scoop instance.
//!
//! This module contains publicly available operations that can be executed on
//! a Scoop session. Certain operations may read or write Scoop's data, hence
//! a session is required to perform these functions.
//!
//! # Note
//!
//! operations with description ending with `*` alter the config.
//!
//! # Examples
//!
//! ```rust
//! use libscoop::{Session, operation};
//! let session = Session::new();
//! let buckets = operation::bucket_list(&session).expect("failed to get buckets");
//! println!("{} bucket(s)", buckets.len());
//! ```
use chrono::{SecondsFormat, Utc};
use futures::{executor::ThreadPool, task::SpawnExt};
use log::debug;
use std::{
    collections::HashSet,
    iter::FromIterator,
    sync::{Arc, Mutex},
};

use crate::{
    bucket::{Bucket, BucketUpdateProgressContext},
    cache::CacheFile,
    error::{Error, Fallible},
    event::Event,
    internal, package,
    package::{InstallInfo, Package, QueryOption},
    Session, SyncOption,
};

/// Add a bucket to Scoop.
///
/// # Errors
///
/// This method will return an error if the bucket already exists, or the remote
/// url is not specified when adding a non built-in bucket.
///
/// A git error will be returned if failed to clone the bucket.
pub fn bucket_add(session: &Session, name: &str, remote_url: &str) -> Fallible<()> {
    let config = session.config();
    let mut path = config.root_path().to_owned();
    path.push("buckets");

    internal::fs::ensure_dir(&path)?;

    path.push(name);
    if path.exists() {
        return Err(Error::BucketAlreadyExists(name.to_owned()));
    }

    let proxy = config.proxy();
    let remote_url = match remote_url.is_empty() {
        false => remote_url,
        true => crate::constant::BUILTIN_BUCKET_LIST
            .iter()
            .find(|&&(n, _)| n == name)
            .map(|&(_, remote)| remote)
            .ok_or_else(|| Error::BucketAddRemoteRequired(name.to_owned()))?,
    };

    internal::git::clone_repo(remote_url, path, proxy)
}

/// Get a list of added buckets.
///
/// # Returns
///
/// A list of added buckets sorted by name.Buckets cannot be parsed will be
/// filtered out.
///
/// # Errors
///
/// I/O errors will be returned if the `buckets` directory is not readable.
pub fn bucket_list(session: &Session) -> Fallible<Vec<Bucket>> {
    crate::bucket::bucket_added(session).map(|mut buckets| {
        buckets.sort_by_key(|b| b.name().to_owned());
        buckets
    })
}

/// Get a list of known (built-in) buckets.
///
/// # Returns
///
/// A list of known buckets.
pub fn bucket_list_known() -> Vec<(&'static str, &'static str)> {
    crate::constant::BUILTIN_BUCKET_LIST.to_vec()
}

/// Update all added buckets. *
///
/// # Errors
///
/// I/O errors will be returned if the `buckets` directory is not readable or
/// failed to start up the update threads.
///
/// A [`ConfigInUse`][1] error will be returned if the config is borrowed elsewhere.
///
/// [1]: crate::Error::ConfigInUse
pub fn bucket_update(session: &Session) -> Fallible<()> {
    let buckets = crate::bucket::bucket_added(session)?;

    if buckets.is_empty() {
        return Ok(());
    }

    // Doing bucket update will update the last_update timestamp in the config.
    // A mutable reference to the config is borrowed here.
    let mut config = session.config_mut()?;
    let any_bucket_updated = Arc::new(Mutex::new(false));
    let mut tasks = Vec::new();
    let pool = ThreadPool::builder().create()?;
    let proxy = config.proxy().map(|s| s.to_owned());
    let emitter = session.emitter();

    for bucket in buckets.iter() {
        let repo = bucket.path().to_owned();

        // There is no remote url for this bucket, so we just ignore it.
        if bucket.remote_url().is_none() {
            debug!("ignored not updatable bucket {}", bucket.name());
            continue;
        }

        let name = bucket.name().to_owned();
        let flag = Arc::clone(&any_bucket_updated);
        let proxy = proxy.clone();
        let emitter = emitter.clone();

        let task = pool
            .spawn_with_handle(async move {
                let mut ctx = BucketUpdateProgressContext::new(name.as_str());

                if let Some(tx) = emitter.clone() {
                    let _ = tx.send(Event::BucketUpdateProgress(ctx.clone()));
                }

                match internal::git::reset_head(repo, proxy) {
                    Ok(_) => {
                        *flag.lock().unwrap() = true;

                        if let Some(tx) = emitter {
                            ctx.set_succeeded();
                            let _ = tx.send(Event::BucketUpdateProgress(ctx));
                        }
                    }
                    Err(err) => {
                        if let Some(tx) = emitter {
                            ctx.set_failed(err.to_string().as_str());
                            let _ = tx.send(Event::BucketUpdateProgress(ctx));
                        }
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
        config.set("last_update", time.as_str())?;
    }

    if let Some(tx) = emitter {
        let _ = tx.send(Event::BucketUpdateDone);
    }
    Ok(())
}

/// Remove a bucket from Scoop.
///
/// # Errors
///
/// This method will return an error if the bucket does not exist. I/O errors
/// will be returned if the bucket directory is unable to be removed.
pub fn bucket_remove(session: &Session, name: &str) -> Fallible<()> {
    let mut path = session.config().root_path().to_owned();
    path.push("buckets");
    path.push(name);

    if !path.exists() {
        return Err(Error::BucketNotFound(name.to_owned()));
    }

    Ok(remove_dir_all::remove_dir_all(path.as_path())?)
}

/// Get a list of downloaded cache files.
///
/// # Returns
///
/// A list of downloaded cache files.
///
/// # Errors
///
/// I/O errors will be returned if the cache directory is not readable.
pub fn cache_list(session: &Session, query: &str) -> Fallible<Vec<CacheFile>> {
    let is_wildcard_query = query.eq("*") || query.is_empty();
    let config = session.config();
    let cache_dir = config.cache_path();
    let mut entries = vec![];

    if cache_dir.exists() {
        entries = cache_dir
            .read_dir()?
            .filter_map(Result::ok)
            .filter_map(|de| {
                let is_file = de.file_type().unwrap().is_file();
                if is_file {
                    if let Ok(item) = CacheFile::from(de.path()) {
                        if !is_wildcard_query {
                            let matched = item
                                .package_name()
                                .to_lowercase()
                                .contains(&query.to_lowercase());
                            if matched {
                                return Some(item);
                            } else {
                                return None;
                            }
                        }

                        return Some(item);
                    }
                }
                None
            })
            .collect::<Vec<_>>();
    }

    Ok(entries)
}

/// Remove cache files by query.
///
/// # Errors
///
/// I/O errors will be returned if the cache directory is not readable or failed
/// to remove the cache files.
pub fn cache_remove(session: &Session, query: &str) -> Fallible<()> {
    match query {
        "*" => {
            let config = session.config();
            Ok(internal::fs::empty_dir(config.cache_path())?)
        }
        query => {
            let files = cache_list(session, query)?;
            for f in files.into_iter() {
                std::fs::remove_file(f.path())?;
            }
            Ok(())
        }
    }
}

/// Get the configuation list.
///
/// # Returns
///
/// A string of the configuation list in pretty-printed JSON format.
///
/// # Errors
///
/// Serde errors will be returned if the config cannot be serialized.
pub fn config_list(session: &Session) -> Fallible<String> {
    let config = session.config();
    config.pretty()
}

/// Set a configuation key. *
///
/// # Errors
///
/// A [`ConfigInUse`][1] error will be returned if the config is borrowed
/// elsewhere.
///
/// A [`ConfigKeyInvalid`][2] error will be returned if the key is invalid.
///
/// A [`ConfigValueInvalid`][3] error will be returned if the value is invalid.
///
/// [1]: crate::Error::ConfigInUse
/// [2]: crate::Error::ConfigKeyInvalid
/// [3]: crate::Error::ConfigValueInvalid
pub fn config_set(session: &Session, key: &str, value: &str) -> Fallible<()> {
    session.config_mut()?.set(key, value)
}

/// Hold or unhold a package.
///
/// # Errors
///
/// This method will return an error if the package is not installed.
///
/// A [`PackageHoldBrokenInstall`][1] error will be returned if the install is
/// broken (`install.json` is missing or broken).
///
/// I/O errors will be returned if failed to write the `install.json` file.
/// Serde errors will be returned if the install info cannot be serialized.
///
/// [1]: crate::Error::PackageHoldBrokenInstall
pub fn package_hold(session: &Session, name: &str, flag: bool) -> Fallible<()> {
    let mut path = session.config().root_path().to_owned();
    path.push("apps");
    path.push(name);

    if !path.exists() {
        return Err(Error::PackageHoldNotInstalled(name.to_owned()));
    }

    path.push("current");
    path.push("install.json");

    if let Ok(mut install_info) = InstallInfo::parse(&path) {
        install_info.set_held(flag);
        internal::fs::write_json(path, install_info)
    } else {
        Err(Error::PackageHoldBrokenInstall(name.to_owned()))
    }
}

/// Query packages.
///
/// # Note
/// Set `installed` to `true` to query installed packages. The returned list
/// will be sorted by package name.
///
/// # Returns
///
/// A list of packages that match the query.
///
/// # Errors
///
/// I/O errors will be returned if the `apps`/`buckets` directory is not readable.
///
/// A [`Regex`][1] error will be returned if the given query is not a valid regex.
///
/// [1]: crate::Error::Regex
pub fn package_query(
    session: &Session,
    queries: Vec<&str>,
    options: Vec<QueryOption>,
    installed: bool,
) -> Fallible<Vec<Package>> {
    let mut packages = vec![];
    // remove possible duplicates
    let mut queries = HashSet::<&str>::from_iter(queries)
        .into_iter()
        .collect::<Vec<_>>();

    if queries.is_empty() {
        queries.push("*");
    }

    packages = if installed {
        package::query::query_installed(session, &queries, &options)?
    } else {
        package::query::query_synced(session, &queries, &options)?
    };

    packages.sort_by_key(|p| p.name().to_owned());

    Ok(packages)
}

/// Sync packages.
///
/// # Note
/// The meaning of `sync` packages is to download, (un)install and/or upgrade
/// packages.
///
/// # Errors
///
/// I/O errors will be returned if the `apps`/`buckets` directory is not readable.
///
/// A [`PackageNotFound`][1] error will be returned if no package is found for
/// the given query.
///
/// A [`PackageMultipleCandidates`][2] error will be returned if multiple
/// candidates are found for the given query and not able to ask for a selection.
///
/// [1]: crate::Error::PackageNotFound
/// [2]: crate::Error::PackageMultipleCandidates
pub fn package_sync(
    session: &Session,
    queries: Vec<&str>,
    options: Vec<SyncOption>,
) -> Fallible<()> {
    // remove possible duplicates
    let queries = HashSet::<&str>::from_iter(queries)
        .into_iter()
        .collect::<Vec<_>>();

    if let Some(tx) = session.emitter() {
        let _ = tx.send(Event::PackageResolveStart);
    }

    let is_op_remove = options.contains(&SyncOption::Remove);
    if is_op_remove {
        package::sync::remove(session, &queries, &options)?;
    } else {
        package::sync::install(session, &queries, &options)?;
    }

    if let Some(tx) = session.emitter() {
        let _ = tx.send(Event::PackageSyncDone);
    }

    Ok(())
}
