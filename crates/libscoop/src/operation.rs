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
    error::{Error, Fallible},
    event::{BucketUpdateFailedCtx, Event},
    internal::{
        self,
        fs::{self, filenamify},
        git,
    },
    package::{self, resolve, InstallInfo, Package, QueryOption},
    Session, SyncOption,
};

/// Add a bucket to Scoop.
pub fn bucket_add(session: &Session, name: &str, remote_url: &str) -> Fallible<()> {
    let config = session.config();
    let mut path = config.root_path.clone();

    path.push("buckets");
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

    git::clone_repo(remote_url, path, proxy)
}

/// Get a list of added buckets.
pub fn bucket_list(session: &Session) -> Fallible<Vec<Bucket>> {
    let mut buckets = Vec::new();
    let buckets_dir = session.config().root_path.join("buckets");

    if buckets_dir.exists() {
        let entries = buckets_dir
            .read_dir()?
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

/// Update all added buckets. *
pub fn bucket_update(session: &Session) -> Fallible<()> {
    let buckets = bucket_list(session)?;

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

        if !repo.join(".git").exists() {
            debug!("ignored non-git bucket {}", bucket.name());
            continue;
        }

        let name = bucket.name().to_owned();
        let flag = Arc::clone(&any_bucket_updated);
        let proxy = proxy.clone();
        let emitter = emitter.clone();

        let task = pool
            .spawn_with_handle(async move {
                if emitter.is_some() {
                    let tx = emitter.clone().unwrap();
                    let _ = tx.send(Event::BucketUpdateStarted(name.clone()));
                }

                match git::reset_head(repo, proxy) {
                    Ok(_) => {
                        *flag.lock().unwrap() = true;
                        if let Some(tx) = emitter {
                            let _ = tx.send(Event::BucketUpdateSuccessed(name));
                        }
                    }
                    Err(err) => {
                        if let Some(tx) = emitter {
                            let ctx: BucketUpdateFailedCtx = BucketUpdateFailedCtx {
                                name: name.clone(),
                                err_msg: err.to_string(),
                            };

                            let _ = tx.send(Event::BucketUpdateFailed(ctx));
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
        let _ = tx.send(Event::BucketUpdateFinished);
    }
    Ok(())
}

/// Remove a bucket from Scoop.
pub fn bucket_remove(session: &Session, name: &str) -> Fallible<()> {
    let mut path = session.config().root_path.clone();
    path.push("buckets");
    path.push(name);

    if !path.exists() {
        return Err(Error::BucketNotFound(name.to_owned()));
    }

    Ok(remove_dir_all::remove_dir_all(path.as_path())?)
}

/// Get a list of downloaded cache files.
pub fn cache_list(session: &Session, query: &str) -> Fallible<Vec<CacheFile>> {
    let mut entires = session
        .config()
        .cache_path
        .read_dir()?
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
        "*" => Ok(fs::empty_dir(&session.config().cache_path)?),
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
pub fn config_list(session: &Session) -> Fallible<String> {
    let config = session.config();
    config.pretty()
}

/// Set a configuation key. *
pub fn config_set(session: &Session, key: &str, value: &str) -> Fallible<()> {
    session.config_mut()?.set(key, value)
}

/// Hold or unhold a package.
pub fn package_hold(session: &Session, name: &str, flag: bool) -> Fallible<()> {
    let mut path = session.config().root_path.clone();
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

/// Query packages.
///
/// Set `installed` to `true` to query installed packages.
pub fn package_query(
    session: &Session,
    queries: Vec<&str>,
    options: Vec<QueryOption>,
    installed: bool,
) -> Fallible<Vec<Package>> {
    let mut ret = vec![];
    // remove possible duplicates
    let mut queries = HashSet::<&str>::from_iter(queries);
    if queries.is_empty() {
        queries.insert("*");
    }

    for query in queries.into_iter() {
        let qret = if installed {
            package::query::query_installed(session, query, &options)?
        } else {
            package::query::query_synced(session, query, &options)?
        };
        ret.extend(qret);
    }

    ret.sort_by_key(|p| p.name().to_owned());

    Ok(ret)
}

/// Sync packages.
///
/// The meaning of `sync` packages is to download, (un)install and/or upgrade
/// packages.
pub fn package_sync(
    session: &Session,
    queries: Vec<&str>,
    options: Vec<SyncOption>,
) -> Fallible<()> {
    let is_op_remove = options.contains(&SyncOption::Remove);
    let query_opts = vec![QueryOption::Explicit];
    let mut packages = vec![];
    // remove possible duplicates
    let queries = HashSet::<&str>::from_iter(queries);

    let emitter = session.emitter();
    if let Some(tx) = emitter.clone() {
        let _ = tx.send(Event::PackageResolveStart);
    }

    for query in queries.into_iter() {
        let mut pkg = if is_op_remove {
            package::query::query_installed(session, query, &query_opts)?
        } else {
            package::query::query_synced(session, query, &query_opts)?
        };

        if pkg.is_empty() {
            return Err(Error::PackageNotFound(query.to_owned()));
        }

        if pkg.len() > 1 {
            resolve::select_candidate(session, &mut pkg)?;
        }

        packages.extend(pkg);
    }

    if is_op_remove {
        resolve::resolve_dependents(session, &mut packages)?;

        println!("The following packages will be removed:");
        for pkg in packages.iter() {
            println!("  {}", pkg.name());
        }
    } else {
        resolve::resolve_dependencies(session, &mut packages)?;

        // filter installed packages
        packages = packages
            .into_iter()
            .filter(|p| !p.is_strictly_installed())
            .collect::<Vec<_>>();

        if packages.is_empty() {
            return Ok(());
        }

        let download_only = options.contains(&SyncOption::DownloadOnly);
        let ignore_cache = options.contains(&SyncOption::IgnoreCache);

        if download_only {
            println!("The following packages will be downloaded:");
        } else {
            println!("The following packages will be installed:");
        }
        for pkg in packages.iter() {
            println!("  {}", pkg.ident());
        }

        if let Some(tx) = emitter {
            let _ = tx.send(Event::PackageDownloadSizingStart);
        }

        let mut total_size = 0f64;
        let mut size_estimated = false;

        let config = session.config();
        let cache_root = config.cache_path.clone();
        let proxy = config.proxy();

        for pkg in packages.iter() {
            let mut urls_mapped_files = pkg
                .url()
                .into_iter()
                .map(|url| {
                    let fname = format!("{}#{}#{}", pkg.name(), pkg.version(), filenamify(url));
                    let path = cache_root.join(fname);
                    (url, path)
                })
                .collect::<Vec<_>>();

            if !ignore_cache {
                urls_mapped_files = urls_mapped_files
                    .into_iter()
                    .filter(|(_, path)| !path.exists())
                    .collect::<Vec<_>>();
            }

            for (mut url, _) in urls_mapped_files.into_iter() {
                if url.contains('#') {
                    url = url.split_once('#').unwrap().0;
                }

                let size = internal::network::get_content_length(url, proxy);
                if size.is_none() {
                    size_estimated = true;
                }
                total_size += size.unwrap_or(1f64);
            }
        }
        if size_estimated {
            println!("  Total download size: {} (estimated)", total_size);
        } else {
            println!("  Total download size: {}", total_size);
        }
    }

    Ok(())
}
