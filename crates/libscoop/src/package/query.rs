use log::debug;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use regex::{Regex, RegexBuilder};
use std::{collections::HashSet, vec};

use crate::{
    bucket::Bucket,
    error::{Context, Fallible},
    internal::compare_versions,
    package::manifest::{InstallInfo, Manifest},
    Session,
};

use super::{InstallState, InstallStateInstalled, Package, QueryOption};

pub(crate) fn query_installed(
    session: &Session,
    queries: HashSet<&str>,
    options: Vec<QueryOption>,
) -> Fallible<Vec<Package>> {
    let root_path = session.get_config().root_path.clone();
    let is_wildcard_query = queries.contains("*") || queries.is_empty();
    let apps_dir = root_path.join("apps");
    // build regex queries
    let mut regex_queries: Vec<(Option<String>, Regex)> = vec![];

    if !is_wildcard_query {
        for q in queries.into_iter() {
            match q.contains("/") {
                false => {
                    let re = RegexBuilder::new(q)
                        .case_insensitive(true)
                        .multi_line(true)
                        .build()?;
                    regex_queries.push((None, re));
                }
                true => {
                    let (bucket_prefix, name) = q.split_once("/").unwrap();
                    let re = RegexBuilder::new(name)
                        .case_insensitive(true)
                        .multi_line(true)
                        .build()?;
                    regex_queries.push((Some(bucket_prefix.to_owned()), re));
                }
            }
        }
    }

    let mut packages = apps_dir
        .read_dir()
        .with_context(|| format!("failed to read dir {}", apps_dir.display()))?
        .into_iter()
        .par_bridge()
        .filter_map(|item| {
            if let Ok(e) = item {
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or_default();
                let filename = e.file_name();
                let name = filename.to_str().unwrap();
                // The name `scoop` is reserved for Scoop, ignore it
                let is_scoop = name == "scoop";
                let manifest_path = e.path().join("current/manifest.json");
                let install_info_path = e.path().join("current/install.json");
                let is_not_broken = manifest_path.exists() && install_info_path.exists();

                if !is_dir || is_scoop || !is_not_broken {
                    return None;
                }

                // Here we can do some pre-filtering by package name, if there
                // isn't any wildcard query and no extra query requested on
                // package description or binaries. This could save some query
                // time by avoiding parsing manifest and install info files.
                let extra_query = options.contains(&QueryOption::Binary)
                    || options.contains(&QueryOption::Description);
                let name_matched = regex_queries.iter().any(|(_, re)| re.is_match(name));

                if !is_wildcard_query && !extra_query && !name_matched {
                    return None;
                }

                if let Ok(manifest) = Manifest::parse(manifest_path) {
                    if let Ok(install_info) = InstallInfo::parse(install_info_path) {
                        // Noted that packages installed via URLs don't have
                        // bucket info in install info file. We mark them as
                        // isolated packages and use `__isolated__` as bucket
                        // name.
                        let bucket = install_info.bucket().unwrap_or("__isolated__");

                        let mut unmatched = true;

                        if is_wildcard_query {
                            unmatched = false;
                        } else {
                            let prefixed_name_matched = regex_queries
                                .iter()
                                .filter(|&(_, re)| re.is_match(name))
                                .any(|(prefix, _)| {
                                    prefix.is_none() || prefix.as_deref().unwrap() == bucket
                                });

                            if prefixed_name_matched {
                                unmatched = false;
                            }

                            if unmatched {
                                if options.contains(&QueryOption::Description) {
                                    let description = manifest.description().unwrap_or_default();
                                    let description_matched = regex_queries
                                        .iter()
                                        .any(|(_, re)| re.is_match(description));
                                    if description_matched {
                                        unmatched = false;
                                    }
                                }

                                if options.contains(&QueryOption::Binary) {
                                    let binaries = manifest.executables().unwrap_or_default();
                                    let binary_matched = regex_queries
                                        .iter()
                                        .any(|(_, re)| binaries.iter().any(|&b| re.is_match(b)));
                                    if binary_matched {
                                        unmatched = false;
                                    }
                                }
                            }
                        }

                        if unmatched {
                            return None;
                        }

                        let current_version = manifest.version().to_owned();

                        let state = InstallState::Installed(InstallStateInstalled {
                            version: current_version.clone(),
                            bucket: install_info.bucket().map(|s| s.to_owned()),
                            arch: install_info.arch().to_owned(),
                            held: install_info.is_held(),
                            url: install_info.url().map(|s| s.to_owned()),
                        });

                        let package = Package::from(name, bucket, manifest);
                        package.fill_install_state(state);

                        // The query has finished, the package has been found
                        // and crafted. We can now apply some extra filters.
                        //
                        // Filter out packages that are not upgradable when
                        // the upgradable option is requested.
                        if options.contains(&QueryOption::Upgradable) {
                            if bucket == "__isolated__" {
                                debug!("ignore isolated package '{}'", name);
                                // isolated packages are not upgradable currently,
                                // we may support it by checking the origin
                                // manifest via the path/url in install info.
                                return None;
                            }

                            let mut bucket_path = root_path.join("buckets");
                            bucket_path.push(&bucket);

                            if let Ok(origin_bucket) = Bucket::from(&bucket_path) {
                                let origin_manifest_path = origin_bucket.path_of_manifest(name);
                                if origin_manifest_path.exists() {
                                    // println!("origin_manifest_path: {:?}", origin_manifest_path);
                                    if let Ok(origin_manifest) =
                                        Manifest::parse(origin_manifest_path)
                                    {
                                        // let current_version = manifest.version();
                                        let origin_version = origin_manifest.version();
                                        let is_upgradable =
                                            compare_versions(origin_version, &current_version)
                                                == std::cmp::Ordering::Greater;
                                        if is_upgradable {
                                            let origin_pkg =
                                                Package::from(name, bucket, origin_manifest);
                                            package.fill_upgradable(origin_pkg);
                                        } else {
                                            // the package is not upgradable,
                                            // since the upgradable option is
                                            // requested, we should skip it.
                                            return None;
                                        }
                                    }
                                } else {
                                    // the package is not upgradable because
                                    // the origin manifest is not found. This
                                    // could happen when the package is deleted
                                    // or deprecated from the origin bucket.
                                    return None;
                                }
                            } else {
                                // the package is not upgradable because the
                                // origin bucket is not reachable. This could
                                // happen when the bucket is removed or renamed.
                                return None;
                            }
                        }

                        return Some(package);
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>();

    packages.sort_by_key(|p| p.name.clone());

    Ok(packages)
}

pub(crate) fn query_synced(
    session: &Session,
    queries: HashSet<&str>,
    options: Vec<QueryOption>,
) -> Fallible<Vec<Package>> {
    let is_wildcard_query = queries.contains("*") || queries.is_empty();
    let buckets = crate::operation::bucket_list(session)?;
    let apps_dir = session.get_config().root_path.join("apps");
    // build regex queries
    let mut regex_queries: Vec<(Option<String>, Regex)> = vec![];

    if !is_wildcard_query {
        for q in queries.into_iter() {
            match q.contains("/") {
                false => {
                    let re = RegexBuilder::new(q)
                        .case_insensitive(true)
                        .multi_line(true)
                        .build()?;
                    regex_queries.push((None, re));
                }
                true => {
                    let (bucket_prefix, name) = q.split_once("/").unwrap();
                    let re = RegexBuilder::new(name)
                        .case_insensitive(true)
                        .multi_line(true)
                        .build()?;
                    regex_queries.push((Some(bucket_prefix.to_owned()), re));
                }
            }
        }
    }

    let mut packages = buckets
        .iter()
        .par_bridge()
        .filter_map(|bucket| {
            if let Ok(manifest_files) = bucket.manifests() {
                let bucket_packages = manifest_files
                    .into_iter()
                    .par_bridge()
                    .filter_map(|path| {
                        let filename = path.file_stem().unwrap();
                        let name = filename.to_str().unwrap();

                        // Here we can do some pre-filtering by package name, if there
                        // isn't any wildcard query and no extra query requested on
                        // package description or binaries. This could save some query
                        // time by avoiding parsing manifest and install info files.
                        let extra_query = options.contains(&QueryOption::Binary)
                            || options.contains(&QueryOption::Description);
                        let name_matched = regex_queries.iter().any(|(_, re)| re.is_match(name));

                        if !is_wildcard_query && !extra_query && !name_matched {
                            return None;
                        }

                        if let Ok(manifest) = Manifest::parse(&path) {
                            let bucket = bucket.name();

                            let mut unmatched = true;

                            if is_wildcard_query {
                                unmatched = false;
                            } else {
                                let prefixed_name_matched = regex_queries
                                    .iter()
                                    .filter(|&(_, re)| re.is_match(name))
                                    .any(|(prefix, _)| {
                                        prefix.is_none() || prefix.as_deref().unwrap() == bucket
                                    });

                                if prefixed_name_matched {
                                    unmatched = false;
                                }

                                if unmatched {
                                    if options.contains(&QueryOption::Description) {
                                        let description =
                                            manifest.description().unwrap_or_default();
                                        let description_matched = regex_queries
                                            .iter()
                                            .any(|(_, re)| re.is_match(description));
                                        if description_matched {
                                            unmatched = false;
                                        }
                                    }

                                    if options.contains(&QueryOption::Binary) {
                                        let binaries = manifest.executables().unwrap_or_default();
                                        let binary_matched = regex_queries.iter().any(|(_, re)| {
                                            binaries.iter().any(|&b| re.is_match(b))
                                        });
                                        if binary_matched {
                                            unmatched = false;
                                        }
                                    }
                                }
                            }

                            if unmatched {
                                return None;
                            }

                            let package = Package::from(name, bucket, manifest);

                            // The query has finished, the package has been found,
                            // the last step is to check if the package's install
                            // state.
                            let mut path = apps_dir.join(name);
                            if path.exists() {
                                path.push("current");
                                path.push("install.json");
                                if let Ok(install_info) = InstallInfo::parse(&path) {
                                    // Will not be considered as installed if
                                    // the bucket name is not matched.
                                    if install_info.bucket().unwrap_or_default() == bucket {
                                        path.pop();
                                        path.push("manifest.json");
                                        if let Ok(install_manifest) = Manifest::parse(path) {
                                            let state =
                                                InstallState::Installed(InstallStateInstalled {
                                                    version: install_manifest.version().to_owned(),
                                                    bucket: install_info
                                                        .bucket()
                                                        .map(|s| s.to_owned()),
                                                    arch: install_info.arch().to_owned(),
                                                    held: install_info.is_held(),
                                                    url: install_info.url().map(|s| s.to_owned()),
                                                });
                                            package.fill_install_state(state);
                                        }
                                    }
                                }
                            } else {
                                package.fill_install_state(InstallState::NotInstalled);
                            }

                            return Some(package);
                        }
                        None
                    })
                    .collect::<Vec<_>>();

                return Some(bucket_packages);
            }
            None
        })
        .flatten()
        .collect::<Vec<_>>();

    packages.sort_by_key(|p| p.name.clone());

    Ok(packages)
}
