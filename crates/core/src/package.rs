use futures::{executor::ThreadPool, task::SpawnExt};
use log::{debug, trace, warn};
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelBridge;
use scoop_hash::Checksum;
use std::{
    collections::HashSet,
    io::{BufReader, Read},
    os::windows::prelude::FileExt,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    sync::{Arc, Mutex},
};
use ureq::{AgentBuilder, Request};

use crate::{
    bucket::Bucket,
    cache::CacheFile,
    constants::REGEX_ARCHIVE_7Z,
    error::{Context, Error, Fallible},
    manifest::{InstallInfo, License, Manifest},
    session::Session,
    util::{compare_versions, dag::DepGraph, ensure_dir, is_program_available},
};

pub type PackageList = Vec<Package>;

#[derive(Clone, Debug)]
pub struct Package {
    pub bucket: String,
    pub name: String,
    pub manifest_hash: u64,
    pub manifest_path: String,
    pub version: String,
    pub description: Option<String>,
    pub homepage: String,
    pub license: License,
    pub dependencies: Option<Vec<String>>,
    pub supported_arch: Vec<String>,
    pub shims: Option<Vec<String>>,
    pub state: PackageState,
    pub manifest: Manifest,
    upgradable_version: Option<String>,
}

#[derive(Clone, Debug)]
pub enum PackageState {
    NotInstalled,
    Installed(InstalledState),
}

#[derive(Clone, Debug)]
pub struct InstalledState {
    pub arch: String,
    pub held: bool,
}

#[derive(Clone, Debug)]
pub struct DownloadProgressContext {
    pub name: String,
    pub total: u64,
    pub position: u64,
    pub file_count: usize,
    pub index: usize,
    pub state: DownloadProgressState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadProgressState {
    Prepared,
    Downloading,
    Finished,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum InstallOption {
    AssumeNo,
    AssumeYes,
    DownloadOnly,
    IgnoreFailure,
    IgnoreHold,
    NoCache,
    NoHashCheck,
    NoUpgrade,
    OnlyUpgrade,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SearchMode {
    Explicit,
    FuzzyDefault,
    FuzzyNamesOnly,
    FuzzyWithBinaries,
    Unique,
}

impl From<InstallInfo> for InstalledState {
    fn from(info: InstallInfo) -> Self {
        let arch = info.arch().to_owned();
        let held = info.held().to_owned();
        InstalledState { arch, held }
    }
}

impl Package {
    pub fn from(
        bucket: String,
        name: String,
        manifest: Manifest,
        install_info: Option<InstallInfo>,
        upgradable_version: Option<String>,
    ) -> Package {
        let description = manifest.description().map(|s| s.to_owned());
        let shims = manifest
            .executables()
            .map(|v| v.into_iter().map(|s| s.to_owned()).collect());
        let homepage = manifest.homepage().to_owned();
        let license = manifest.license().to_owned();
        let manifest_hash = 0u64;
        let manifest_path = manifest.path().to_owned();
        let dependencies = manifest
            .raw_dependencies()
            .map(|v| v.into_iter().map(|s| s.to_owned()).collect());
        let state = match install_info {
            None => PackageState::NotInstalled,
            Some(info) => PackageState::Installed(info.into()),
        };
        let supported_arch = manifest.supported_arch();
        let version = manifest.version().to_owned();
        Package {
            bucket,
            dependencies,
            description,
            shims,
            homepage,
            license,
            manifest_hash,
            manifest_path,
            name,
            state,
            supported_arch,
            version,
            manifest,
            upgradable_version,
        }
    }

    /// Return the identity of this package, in the form of `bucket/name`, which
    /// is unique for each package
    #[inline]
    pub fn ident(&self) -> String {
        format!("{}/{}", self.bucket, self.name)
    }

    #[inline]
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    #[inline]
    pub fn homepage(&self) -> &str {
        &self.homepage
    }

    #[inline]
    pub fn license(&self) -> &License {
        &self.license
    }

    #[inline]
    pub fn installed(&self) -> bool {
        match self.state {
            PackageState::NotInstalled => false,
            PackageState::Installed(_) => true,
        }
    }

    #[inline]
    pub fn is_held(&self) -> bool {
        match self.state {
            PackageState::NotInstalled => false,
            PackageState::Installed(ref info) => info.held,
        }
    }

    /// Check if the package is upgradable. Return the upgradable version when
    /// it is.
    #[inline]
    pub fn upgradable(&self) -> Option<&str> {
        self.upgradable_version.as_ref().map(|s| s.as_str())
    }

    #[inline]
    pub fn shims(&self) -> Option<Vec<&str>> {
        self.shims
            .as_ref()
            .map(|e| e.iter().map(|s| s.as_str()).collect())
    }
}

pub fn search_installed_packages(
    session: &Session,
    queries: HashSet<&str>,
    upgradable: bool,
) -> Fallible<PackageList> {
    // let mode = SearchMode::FuzzyNamesOnly;
    let query_all = queries.contains("*") || queries.is_empty();

    let apps_dir = session.config.root_path.join("apps");
    let mut packages = apps_dir
        .read_dir()
        .with_context(|| format!("failed to read dir: {}", apps_dir.display()))?
        .into_iter()
        .par_bridge()
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_str().map(|n| n.to_string());
            if name.is_none() {
                return None;
            }
            let name = name.unwrap();

            // mode == SearchMode::FuzzyNamesOnly
            if !query_all {
                let found = queries.iter().any(|&q| match q.contains("/") {
                    false => name.contains(q),
                    true => q
                        .split_once("/")
                        .map(|(_, n)| name.contains(n))
                        .unwrap_or_default(),
                });
                if !found {
                    return None;
                }
            }

            let manifest_path = e.path().join("current/manifest.json");
            let install_info_path = e.path().join("current/install.json");

            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or_default();
            // Skip scoop itself
            let is_not_scoop = &name != "scoop";
            let has_manifest = manifest_path.exists();
            let has_install = install_info_path.exists();

            if is_dir && is_not_scoop && has_manifest && has_install {
                if let Ok(manifest) = Manifest::parse(manifest_path) {
                    if let Ok(install_info) = InstallInfo::parse(install_info_path) {
                        let bucket = install_info.bucket().to_owned();
                        let install_info = Some(install_info);
                        // Skip packages installed via URLs
                        if bucket.is_empty() {
                            debug!("ignore isolated package '{}'", name);
                            return None;
                        }

                        // Match buckets
                        if !query_all {
                            let bucket_matched = queries.iter().any(|&q| match q.contains("/") {
                                false => true,
                                true => q
                                    .split_once("/")
                                    .map(|(b, _)| b == &bucket)
                                    .unwrap_or_default(),
                            });
                            if !bucket_matched {
                                return None;
                            }
                        }

                        // check upgradable
                        let mut upgradable_version = None;
                        if upgradable {
                            let bucket_path =
                                session.config.root_path.join("buckets").join(&bucket);
                            if let Ok(up_bucket) = Bucket::from(&bucket_path) {
                                let up_manifest_path = up_bucket.path_of_manifest(&name);
                                // println!("up_manifest_path: {:?}", up_manifest_path);
                                if let Ok(up_manifest) = Manifest::parse(up_manifest_path) {
                                    let version = manifest.version();
                                    let up_version = up_manifest.version();
                                    let is_upgradable = compare_versions(up_version, version)
                                        == std::cmp::Ordering::Greater;
                                    if is_upgradable {
                                        upgradable_version = Some(up_version.to_owned());
                                    } else {
                                        return None;
                                    }
                                }
                            }
                        }

                        let package =
                            Package::from(bucket, name, manifest, install_info, upgradable_version);
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

pub fn search_available_packages(
    session: &Session,
    queries: HashSet<&str>,
    mode: SearchMode,
) -> Fallible<PackageList> {
    let buckets = crate::bucket::bucket_list(session)?;
    let mut not_found_queries = Vec::<String>::new();
    let mut multi_records_queries = Vec::<(String, PackageList)>::new();
    let mut packages = Vec::<Package>::new();

    for query in queries.into_iter() {
        match query.contains("/") {
            true => {
                let (query_bucket, query_name) = query.split_once("/").unwrap();
                let wildcard_query = query_name == "*" || query_name.is_empty();
                if wildcard_query && mode == SearchMode::Unique {
                    return Err(Error::Custom(
                        "wildcard query are not allowed in unique search mode".into(),
                    ));
                }

                let bucket_names = buckets
                    .iter()
                    .enumerate()
                    .map(|(idx, b)| (idx, b.name()))
                    .collect::<Vec<_>>();
                let idx = bucket_names
                    .into_iter()
                    .find(|(_, name)| name == &query_bucket);

                if let Some((idx, _)) = idx {
                    let ret = query_from_bucket(query_name, &buckets[idx], mode)?;
                    match ret.len() {
                        0 => not_found_queries.push(query.to_string()),
                        _ => packages.extend(ret),
                    }
                } else {
                    not_found_queries.push(query.to_string());
                }
            }
            false => {
                let mut records = Vec::<Package>::new();

                let ret = buckets
                    .iter()
                    .par_bridge()
                    .map(|bucket| -> Fallible<PackageList> {
                        query_from_bucket(query, bucket, mode)
                    })
                    .collect::<Vec<_>>();
                for r in ret {
                    records.extend(r?);
                }

                match records.len() {
                    0 => not_found_queries.push(query.to_string()),
                    1 => {
                        if not_found_queries.is_empty() {
                            packages.extend(records);
                        }
                    }
                    _ => {
                        if not_found_queries.is_empty() && mode != SearchMode::Unique {
                            packages.extend(records);
                        } else {
                            records.sort_by_key(|p| p.ident().clone());
                            multi_records_queries.push((query.to_string(), records));
                        }
                    }
                }
            }
        }
    }

    // By default the search for queries is an AND operation, thus if any of
    // the queries is not found, the entire search fails.
    if !not_found_queries.is_empty() {
        let kind = Error::PackageNotFound {
            queries: not_found_queries,
        };
        return Err(kind.into());
    }

    // Return PackageMultipleRecordsFound error if search mode is Unique
    // and there are multiple records found for a query.
    if mode == SearchMode::Unique {
        if !multi_records_queries.is_empty() {
            let kind = Error::PackageMultipleRecordsFound {
                records: multi_records_queries,
            };
            return Err(kind.into());
        }
    }

    packages.sort_by_key(|p| p.ident().clone());
    Ok(packages)
}

fn query_from_bucket(query: &str, bucket: &Bucket, mode: SearchMode) -> Fallible<PackageList> {
    let query_all = query == "*" || query.is_empty();

    let manifest_files = bucket.manifests()?;
    let ret = manifest_files
        .into_iter()
        .par_bridge()
        .filter_map(|path| {
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();

            if !query_all {
                match mode {
                    SearchMode::Explicit | SearchMode::Unique => {
                        if name != query {
                            return None;
                        }
                    }
                    SearchMode::FuzzyNamesOnly => {
                        if !name.contains(query) {
                            return None;
                        }
                    }
                    _ => {
                        // no-op
                    }
                }
            }

            let manifest = Manifest::parse(path);
            if manifest.is_err() {
                warn!("{:?}", manifest);
                return None;
            }
            let manifest = manifest.unwrap();

            let bucket = bucket.name().to_owned();
            let description = manifest.description().map(|s| s.to_lowercase());
            let shims = manifest
                .executables()
                .map(|v| v.into_iter().map(|s| s.to_owned()).collect());

            // Secondary filtering
            if !query_all {
                match mode {
                    SearchMode::FuzzyDefault | SearchMode::FuzzyWithBinaries => {
                        let name_matched = name.contains(query);
                        let desc_matched = description
                            .as_ref()
                            .map(|s| s.contains(&query.to_lowercase()))
                            .unwrap_or(false);

                        if mode == SearchMode::FuzzyDefault && !name_matched && !desc_matched {
                            return None;
                        }

                        let shim_matched = shims
                            .as_ref()
                            .map(|v: &Vec<String>| v.iter().any(|s| s.contains(query)))
                            .unwrap_or(false);
                        if !(name_matched || desc_matched || shim_matched) {
                            return None;
                        }
                    }
                    _ => {}
                }
            }

            let package = Package::from(bucket, name, manifest, None, None);
            Some(package)
        })
        .collect();

    Ok(ret)
}

pub fn install_packages<F>(
    session: &Session,
    packages: PackageList,
    options: HashSet<InstallOption>,
    callback: F,
) -> Fallible<()>
where
    F: FnMut(DownloadProgressContext) + Send + 'static,
{
    let no_cache = options.contains(&InstallOption::NoCache);
    // let ignore_failure = options.contains(&InstallOption::IgnoreFailure);

    // download
    download_packages(session, &packages, no_cache, callback)?;
    let download_only = options.contains(&InstallOption::DownloadOnly);
    if download_only {
        return Ok(());
    }

    // verrify integrity
    let no_hash_check = options.contains(&InstallOption::NoHashCheck);
    if !no_hash_check {
        verify_integrity(session, &packages)?;
    }

    // setup working_dir, copy cached files to working_dir, do decompression
    for package in &packages {
        let extract_dirs = package.manifest.extract_dir();
        let extract_tos = package.manifest.extract_to();

        let (working_dir, files) = setup_working_dir(session, package)?;
    }

    Ok(())
}

fn setup_working_dir(session: &Session, package: &Package) -> Fallible<(PathBuf, Vec<PathBuf>)> {
    let files = package
        .manifest
        .url()
        .into_iter()
        .map(|url| {
            session.config.cache_path.join(format!(
                "{}#{}#{}",
                package.name,
                package.version,
                crate::util::filenamify(url)
            ))
        })
        .collect::<Vec<_>>();

    let version = match package.manifest.is_nightly() {
        false => package.manifest.version().to_owned(),
        true => {
            let date = chrono::Local::now().format("%Y%m%d");
            format!("nightly-{}", date)
        }
    };

    let working_dir = session
        .config
        .root_path
        .join(format!("apps/{}/{}", package.name, version));
    ensure_dir(&working_dir)
        .with_context(|| format!("failed to create working dir: {}", working_dir.display()))?;

    for src in files.iter() {
        let dst = working_dir.join(src.file_name().unwrap());
        std::fs::copy(&src, &dst)
            .with_context(|| format!("failed to copy file: {}", src.display()))?;
    }

    let ret = (working_dir, files);

    // Return the last file as the fname
    Ok(ret)
}

pub fn resolve_packages(session: &Session, queries: HashSet<&str>) -> Fallible<PackageList> {
    let mode = SearchMode::Unique;
    let mut graph = DepGraph::<String>::new();
    let mut ret = search_available_packages(session, queries, mode)?;
    let mut dep_pkgs = PackageList::new();
    for pkg in ret.iter() {
        dep_pkgs.extend(visit_deps(&mut graph, pkg, session, mode)?);
    }
    ret.extend(dep_pkgs);

    let order = graph.walk_flatten()?;
    trace!("dep graph order: {:?}", order);

    // Sort packages by dependency order
    ret.sort_by(|a, b| {
        let a_idx = order.iter().position(|x| x == &a.ident()).unwrap();
        let b_idx = order.iter().position(|x| x == &b.ident()).unwrap();
        a_idx.cmp(&b_idx)
    });

    Ok(ret)
}

/// Recursively visit dependencies of a package and do cyclic dependencies check
fn visit_deps(
    graph: &mut DepGraph<String>,
    pkg: &Package,
    session: &Session,
    mode: SearchMode,
) -> Fallible<PackageList> {
    let mut ret = PackageList::new();
    match &pkg.dependencies {
        None => graph.register_node(pkg.ident()),
        Some(deps) => {
            let queries = deps.iter().map(|d| d.as_str()).collect();
            ret = search_available_packages(session, queries, mode)?;

            // Cyclic dependencies check
            let dep_nodes = ret.iter().map(|p| p.ident()).collect::<Vec<_>>();
            graph.register_deps(pkg.ident(), dep_nodes);
            graph.check()?;

            let mut dep_pkgs = PackageList::new();
            for pkg in ret.iter() {
                dep_pkgs.extend(visit_deps(graph, pkg, session, mode)?);
            }
            ret.extend(dep_pkgs);
        }
    }
    Ok(ret)
}

fn resolve_package_dependencies(package: &Package) -> Fallible<HashSet<String>> {
    let mut deps = HashSet::new();

    if let Some(depends) = package.manifest.raw_dependencies() {
        deps.extend(depends.into_iter().map(|d| d.to_owned()));
    }

    let url = package.manifest.url();
    let pre_install = package.manifest.pre_install().unwrap_or_default();
    let installer_script = package
        .manifest
        .installer()
        .map(|i| i.script().unwrap_or_default())
        .unwrap_or_default();
    let post_install = package.manifest.post_install().unwrap_or_default();
    let scripts = [pre_install, installer_script, post_install];

    // main/7zip
    if !is_program_available("7z.exe") {
        let archive_7z = url.iter().any(|u| REGEX_ARCHIVE_7Z.is_match(u));
        let script_7z = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-7zipArchive")));
        if archive_7z || script_7z {
            deps.insert("main/7zip".to_owned());
        }
    }

    // main/dark
    if !is_program_available("dark.exe") {
        let script_dark = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-DarkArchive")));
        if script_dark {
            deps.insert("main/dark".to_owned());
        }
    }

    // main/lessmsi
    if !is_program_available("lessmsi.exe") {
        let archive_msi = url.iter().any(|u| u.ends_with(".msi"));
        let script_msi = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-MsiArchive")));
        if archive_msi || script_msi {
            deps.insert("main/lessmsi".to_owned());
        }
    }

    // main/innounp
    if !is_program_available("innounp.exe") {
        let explicit_innounp = package.manifest.innosetup();
        let script_innounp = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-InnoArchive")));
        if explicit_innounp || script_innounp {
            deps.insert("main/innounp".to_owned());
        }
    }

    // main/zstd
    if !is_program_available("zstd.exe") {
        let archive_msi = url.iter().any(|u| u.ends_with(".zst"));
        let script_msi = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-ZstdArchive")));
        if archive_msi || script_msi {
            deps.insert("main/zstd".to_owned());
        }
    }

    Ok(deps)
}

#[derive(Debug)]
struct ChunkedRange {
    pub offset: u64,
    pub length: u64,
    pub data: [u8; 4096],
}

pub fn download_packages<F>(
    session: &Session,
    packages: &PackageList,
    no_cache: bool,
    callback: F,
) -> Fallible<()>
where
    F: FnMut(DownloadProgressContext) + Send + 'static,
{
    let callback = Arc::new(Mutex::new(callback));

    let mut client = AgentBuilder::new();
    client = client.user_agent(crate::constants::USER_AGENT);
    if session.config.proxy().is_some() {
        let proxy = session.config.proxy().unwrap();
        let proxy = ureq::Proxy::new(proxy)
            .with_context(|| format!("failed to parse proxy url: {}", proxy))?;
        client = client.proxy(proxy);
    }
    let client = client.build();

    for package in packages {
        let urls = package.manifest.url();
        let cookie = package.manifest.cookie();

        let file_count = urls.len();

        for (index, url) in urls.into_iter().enumerate() {
            let index = index + 1;
            let client = client.clone();
            let cache_path = session.config.cache_path.join(format!(
                "{}#{}#{}",
                package.name,
                package.version,
                crate::util::filenamify(url)
            ));

            if !no_cache && cache_path.exists() {
                continue;
            }

            // strip `#/dl.7z` url renaming
            let url = url.split_once('#').map(|s| s.0).unwrap_or(url);

            let mut request = client.get(url);

            // Add cookie header if present
            if let Some(cookie) = cookie {
                let mut cookies = vec![];
                for (key, value) in cookie {
                    cookies.push(format!("{}={}", key, value));
                }
                let cookie = cookies.join("; ");
                request = request.set("Cookie", &cookie);
            }

            let response = request
                .call()
                .with_context(|| format!("failed to fetch {}", url))?;

            if response.status() != 200 {
                let message = format!(
                    "failed to fetch {} (status code: {})",
                    url,
                    response.status()
                );
                let source = None;
                return Err(Error::Http { message, source });
            }

            let content_length = response
                .header("Content-Length")
                .map(|s| s.parse::<u64>().unwrap_or_default())
                .unwrap_or_default();
            let accept_ranges = response
                .header("Accept-Ranges")
                .map(|s| "bytes" == s)
                .unwrap_or_default();

            let cache_file = CacheFile::from(cache_path)?;

            let ctx = DownloadProgressContext {
                name: package.ident(),
                total: content_length,
                position: 0,
                file_count,
                index,
                state: DownloadProgressState::Prepared,
            };

            let (tx, rx) = mpsc::channel::<ChunkedRange>();
            let mut tasks = vec![];
            if !accept_ranges {
                let pool = ThreadPool::builder()
                    .pool_size(2)
                    .create()
                    .with_context(|| "failed to create thread pool".into())?;

                let write_task = pool
                    .spawn_with_handle(do_write(cache_file, ctx, rx, callback.clone()))
                    .map_err(|e| Error::Custom(e.to_string()))?;
                tasks.push(write_task);

                let read_task = pool
                    .spawn_with_handle(do_read(response, tx.clone()))
                    .map_err(|e| Error::Custom(e.to_string()))?;
                tasks.push(read_task);
            } else {
                let default_connections = 5;
                let split_size = 5_000_000 as u64;

                let x = content_length;
                let y = split_size;

                let split_count = (x / y + (x % y != 0) as u64) as usize;
                let connections = std::cmp::min(split_count, default_connections);

                let mut ranges = vec![];
                let mut range_start = 0;
                let mut range_end = 0;
                for _ in 1..=split_count {
                    range_end += split_size;
                    if range_end >= content_length {
                        range_end = content_length - 1;
                    }
                    ranges.push((range_start, range_end));
                    range_start = range_end + 1;
                }

                let pool_size = connections + 1;
                let pool = ThreadPool::builder()
                    .pool_size(pool_size)
                    .create()
                    .with_context(|| "failed to create thread pool".into())?;

                let write_task = pool
                    .spawn_with_handle(do_write(cache_file, ctx, rx, callback.clone()))
                    .map_err(|e| Error::Custom(e.to_string()))?;
                tasks.push(write_task);

                for range in ranges {
                    let mut request = client.get(url);
                    request = request.set("Range", &format!("bytes={}-{}", range.0, range.1));
                    let read_task = pool
                        .spawn_with_handle(do_read_range(request, range, tx.clone()))
                        .map_err(|e| Error::Custom(e.to_string()))?;
                    tasks.push(read_task);
                }
            }
            drop(tx);

            let joined = futures::future::join_all(tasks);
            futures::executor::block_on(joined);
        }
    }

    Ok(())
}

async fn do_write<F>(
    cache_file: CacheFile,
    mut ctx: DownloadProgressContext,
    rx: Receiver<ChunkedRange>,
    callback: Arc<Mutex<F>>,
) -> Fallible<()>
where
    F: FnMut(DownloadProgressContext),
{
    let mut callback = callback.lock().unwrap();

    let fd = std::fs::OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(cache_file.path())
        .with_context(|| format!("failed to open cache file: {}", cache_file.path().display()))?;

    // emit
    callback(ctx.clone());

    while let Ok(chunk) = rx.recv() {
        let _ = fd
            .seek_write(&chunk.data[..chunk.length as usize], chunk.offset)
            .with_context(|| {
                format!(
                    "failed to write to cache file: {}",
                    cache_file.path().display()
                )
            })
            .unwrap();

        ctx.position = ctx.position + chunk.length;
        if ctx.state != DownloadProgressState::Downloading {
            ctx.state = DownloadProgressState::Downloading;
        }
        callback(ctx.clone());
    }
    drop(fd);

    ctx.state = DownloadProgressState::Finished;
    // emit
    callback(ctx);
    Ok(())
}

async fn do_read(response: ureq::Response, tx: Sender<ChunkedRange>) -> Fallible<()> {
    let mut chunk = [0; 4096];
    let mut offset = 0;
    let mut reader = response.into_reader();

    loop {
        match reader
            .read(&mut chunk)
            .with_context(|| "failed to read response stream".into())?
        {
            0 => break,
            len => {
                let chunk = ChunkedRange {
                    offset,
                    length: len as u64,
                    data: chunk,
                };
                offset += len as u64;
                tx.send(chunk).unwrap();
            }
        }
    }
    Ok(drop(tx))
}

async fn do_read_range(
    request: Request,
    range: (u64, u64),
    tx: Sender<ChunkedRange>,
) -> Fallible<()> {
    let response = request.call().with_context(|| "failed to fetch".into())?;
    if !(response.status() >= 200 && response.status() <= 299) {
        let message = format!("failed to fetch (status code: {})", response.status());
        let source = None;
        return Err(Error::Http { message, source });
    }

    let mut chunk = [0; 4096];
    let mut offset = range.0;
    let mut reader = BufReader::new(response.into_reader());

    loop {
        match reader
            .read(&mut chunk)
            .with_context(|| "failed to read response stream".into())?
        {
            0 => break,
            length => {
                let chunked_range = ChunkedRange {
                    offset,
                    length: length as u64,
                    data: chunk,
                };

                tx.send(chunked_range)
                    .map_err(|e| Error::Custom(format!("failed to send chunk: {}", e)))?;

                offset += length as u64;
            }
        }
    }
    Ok(drop(tx))
}

pub fn verify_integrity(session: &Session, packages: &PackageList) -> Fallible<()> {
    println!("Verifying integrity of packages...");

    for package in packages {
        // skip nightly package
        if package.manifest.is_nightly() {
            continue;
        }

        let urls = package.manifest.url_with_hash();
        print!("Checking hash of {}... ", package.name);

        for (url, hash) in urls.into_iter() {
            let cache_path = session.config.cache_path.join(format!(
                "{}#{}#{}",
                package.name,
                package.version,
                crate::util::filenamify(url)
            ));

            let mut hasher = Checksum::new(hash).map_err(|e| Error::Custom(e.to_string()))?;
            let mut file = std::fs::File::open(&cache_path)
                .with_context(|| format!("failed to open cache file: {}", cache_path.display()))?;
            let mut buffer = [0; 4096];
            loop {
                let len = file.read(&mut buffer).with_context(|| {
                    format!("failed to read cache file: {}", cache_path.display())
                })?;
                match len {
                    0 => break,
                    len => hasher.consume(&buffer[..len]),
                }
            }
            let checksum = hasher.result();
            if hash != &checksum {
                println!("Err");
                return Err(Error::Custom(format!(
                    "checksum mismatch: {}\n Expected: {}\n Actual: {}",
                    cache_path.display(),
                    hash,
                    checksum
                )));
            }
        }
        println!("Ok");
    }

    Ok(())
}
