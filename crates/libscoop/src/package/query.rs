use log::debug;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use regex::{Regex, RegexBuilder};

use crate::{
    bucket::Bucket,
    constant::ISOLATED_PACKAGE_BUCKET,
    error::Fallible,
    internal::{self, compare_versions},
    package::manifest::{InstallInfo, Manifest},
    Session,
};

use super::{InstallState, InstallStateInstalled, Package};

/// Options that may be used to query Scoop packages.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum QueryOption {
    /// Enable query through package binaries.
    Binary,

    /// Enable query through package description.
    Description,

    /// Explicit mode. Regex is disabled in this mode.
    ///
    /// Query will be performed through the package name only. `Description`
    /// and `Binary` options will be ignored.
    Explicit,

    /// Additionally check if the matched package is upgradable.
    ///
    /// This option only takes effect on querying installed packages.
    Upgradable,
}

/// A trait represents a matcher that can be used to do string matching.
trait Matcher {
    fn is_match(&self, s: &str) -> bool;
}

/// A matcher that does explicit match.
struct ExplicitMatcher<'a>(&'a str);

/// A matcher that does regex match.
struct RegexMatcher(Regex);

impl Matcher for ExplicitMatcher<'_> {
    fn is_match(&self, s: &str) -> bool {
        self.0 == s
    }
}

impl Matcher for RegexMatcher {
    fn is_match(&self, s: &str) -> bool {
        self.0.is_match(s)
    }
}

/// Search installed packages.
pub(crate) fn query_installed(
    session: &Session,
    queries: &[&str],
    options: &[QueryOption],
) -> Fallible<Vec<Package>> {
    let is_explicit_mode = options.contains(&QueryOption::Explicit);
    let is_wildcard_query = queries.contains(&"*") || queries.is_empty();
    let root_path = session.config().root_path().to_owned();
    let apps_dir = root_path.join("apps");
    // build matchers
    let mut matchers: Vec<(Option<String>, Box<dyn Matcher + Send + Sync>)> = vec![];

    if !is_wildcard_query {
        for query in queries {
            let (bucket_prefix, name) = query
                .split_once('/')
                .map(|(b, n)| (Some(b.to_owned()), n))
                .unwrap_or((None, query));

            if is_explicit_mode {
                matchers.push((bucket_prefix, Box::new(ExplicitMatcher(name))));
            } else {
                let re = RegexBuilder::new(name)
                    .case_insensitive(true)
                    .multi_line(true)
                    .build()?;
                matchers.push((bucket_prefix, Box::new(RegexMatcher(re))));
            }
        }
    }

    internal::fs::ensure_dir(&apps_dir)?;

    let packages = apps_dir
        .read_dir()?
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
                let name_matched = if is_wildcard_query {
                    // name is always matched for wildcard query
                    true
                } else {
                    matchers.iter().any(|(_, m)| m.is_match(name))
                };

                if !is_wildcard_query && !extra_query && !name_matched {
                    return None;
                }

                if let Ok(manifest) = Manifest::parse(manifest_path) {
                    if let Ok(install_info) = InstallInfo::parse(install_info_path) {
                        // Noted that packages installed via URLs don't have
                        // bucket info in install info file. We mark them as
                        // isolated packages and use `ISOLATED_PACKAGE_BUCKET`
                        // as bucket name.
                        let bucket = install_info.bucket().unwrap_or(ISOLATED_PACKAGE_BUCKET);

                        let mut unmatched = true;

                        if is_wildcard_query {
                            unmatched = false;
                        } else {
                            let prefixed_name_matched = matchers
                                .iter()
                                .filter(|&(_, m)| m.is_match(name))
                                .any(|(prefix, _)| {
                                    // either no bucket prefix or the bucket
                                    // is also matched.
                                    prefix.is_none() || prefix.as_deref().unwrap() == bucket
                                });

                            if prefixed_name_matched {
                                unmatched = false;
                            }

                            if unmatched && !is_explicit_mode {
                                if options.contains(&QueryOption::Description) {
                                    let description = manifest.description().unwrap_or_default();
                                    let description_matched =
                                        matchers.iter().any(|(_, m)| m.is_match(description));
                                    if description_matched {
                                        unmatched = false;
                                    }
                                }

                                if options.contains(&QueryOption::Binary) {
                                    let binaries = manifest.executables().unwrap_or_default();
                                    let binary_matched = matchers
                                        .iter()
                                        .any(|(_, m)| binaries.iter().any(|&b| m.is_match(b)));
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
                        package.fill_install_state(state.clone());

                        // The query has finished, the package has been found
                        // and crafted. We can now apply some extra filters.
                        //
                        // Filter out packages that are not upgradable when
                        // the upgradable option is requested.
                        if options.contains(&QueryOption::Upgradable) {
                            if bucket == ISOLATED_PACKAGE_BUCKET {
                                debug!("ignore isolated package '{}'", name);
                                // isolated packages are not upgradable currently,
                                // we may support it by live checking the origin
                                // manifest via the path/url in install_info.
                                return None;
                            }

                            let mut bucket_path = root_path.join("buckets");
                            bucket_path.push(bucket);

                            if let Ok(origin_bucket) = Bucket::from(&bucket_path) {
                                if let Some(origin_manifest_path) =
                                    origin_bucket.path_of_manifest(name)
                                {
                                    if let Ok(origin_manifest) =
                                        Manifest::parse(origin_manifest_path)
                                    {
                                        let origin_version = origin_manifest.version();
                                        let is_upgradable =
                                            compare_versions(origin_version, &current_version)
                                                == std::cmp::Ordering::Greater;
                                        if is_upgradable {
                                            let origin_pkg =
                                                Package::from(name, bucket, origin_manifest);
                                            origin_pkg.fill_install_state(state);

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

    Ok(packages)
}

/// Search available packages.
pub(crate) fn query_synced(
    session: &Session,
    queries: &[&str],
    options: &[QueryOption],
) -> Fallible<Vec<Package>> {
    let is_explicit_mode = options.contains(&QueryOption::Explicit);
    let is_wildcard_query = queries.contains(&"*") || queries.is_empty();
    let buckets = crate::bucket::bucket_added(session)?;
    let apps_dir = session.config().root_path().join("apps");
    // build matchers
    let mut matchers: Vec<(Option<String>, Box<dyn Matcher + Send + Sync>)> = vec![];

    if !is_wildcard_query {
        for query in queries {
            let (bucket_prefix, name) = query
                .split_once('/')
                .map(|(b, n)| (Some(b.to_owned()), n))
                .unwrap_or((None, query));

            if is_explicit_mode {
                matchers.push((bucket_prefix, Box::new(ExplicitMatcher(name))));
            } else {
                let re = RegexBuilder::new(name)
                    .case_insensitive(true)
                    .multi_line(true)
                    .build()?;
                matchers.push((bucket_prefix, Box::new(RegexMatcher(re))));
            }
        }
    }

    let packages = buckets
        .iter()
        .par_bridge()
        .filter_map(|bucket| {
            if let Ok(manifest_files) = bucket.manifests() {
                let bucket_packages = manifest_files
                    .into_iter()
                    .par_bridge()
                    .filter_map(|entry| {
                        let filename = entry.file_name();
                        let name = filename.to_str().unwrap().strip_suffix(".json").unwrap();

                        // Here we can do some pre-filtering by package name, if there
                        // isn't any wildcard query and no extra query requested on
                        // package description or binaries. This could save some query
                        // time by avoiding parsing manifest and install info files.
                        let extra_query = options.contains(&QueryOption::Binary)
                            || options.contains(&QueryOption::Description);
                        let name_matched = if is_wildcard_query {
                            // name is always matched for wildcard query
                            true
                        } else {
                            matchers.iter().any(|(_, m)| m.is_match(name))
                        };

                        if !is_wildcard_query && !extra_query && !name_matched {
                            return None;
                        }

                        if let Ok(manifest) = Manifest::parse(entry.path()) {
                            let bucket = bucket.name();

                            let mut unmatched = true;

                            if is_wildcard_query {
                                unmatched = false;
                            } else {
                                let prefixed_name_matched = matchers
                                    .iter()
                                    .filter(|&(_, m)| m.is_match(name))
                                    .any(|(prefix, _)| {
                                        // either no bucket prefix or the bucket
                                        // is also matched.
                                        prefix.is_none() || prefix.as_deref().unwrap() == bucket
                                    });

                                if prefixed_name_matched {
                                    unmatched = false;
                                }

                                if unmatched && !is_explicit_mode {
                                    if options.contains(&QueryOption::Description) {
                                        let description =
                                            manifest.description().unwrap_or_default();
                                        let description_matched =
                                            matchers.iter().any(|(_, m)| m.is_match(description));
                                        if description_matched {
                                            unmatched = false;
                                        }
                                    }

                                    if options.contains(&QueryOption::Binary) {
                                        let binaries = manifest.executables().unwrap_or_default();
                                        let binary_matched = matchers
                                            .iter()
                                            .any(|(_, m)| binaries.iter().any(|&b| m.is_match(b)));
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
                            // the last step is to check if the package is installed.
                            let mut path = apps_dir.join(name);
                            path.push("current");
                            path.push("install.json");

                            if let Ok(install_info) = InstallInfo::parse(&path) {
                                path.pop();
                                path.push("manifest.json");
                                if let Ok(install_manifest) = Manifest::parse(path) {
                                    let state = InstallState::Installed(InstallStateInstalled {
                                        version: install_manifest.version().to_owned(),
                                        bucket: install_info.bucket().map(|s| s.to_owned()),
                                        arch: install_info.arch().to_owned(),
                                        held: install_info.is_held(),
                                        url: install_info.url().map(|s| s.to_owned()),
                                    });
                                    package.fill_install_state(state);
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

    Ok(packages)
}
