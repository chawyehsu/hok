use curl::easy::{Easy, List};
use curl::multi::Multi;
use flume::Sender;
use lazycell::LazyCell;
use log::debug;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Write,
    time::Duration,
};

use crate::{constant::DEFAULT_USER_AGENT, error::Fallible, Event, Session};

use super::Package;

/// Download size information.
#[derive(Clone, Copy)]
pub struct DownloadSize {
    /// Total size to download.
    pub total: u64,

    /// Whether the total size is estimated.
    pub estimated: bool,
}

/// A set of packages to download.
pub struct PackageSet<'a> {
    /// Associated libscoop session.
    session: &'a Session,

    /// Packages with intent to download.
    packages: &'a [&'a Package],

    /// Multi handle for curl.
    multi: Multi,

    caches: LazyCell<HashMap<String, PackageCache<'a>>>,

    /// Whether to reuse cached files.
    reuse_cache: bool,
}

/// Stores download information of a file.
struct FileDownloadInfo<'a> {
    /// Download URL.
    url: &'a str,

    /// Local cached file size.
    local_size: u64,

    /// Remote file size.
    remote_size: u64,

    /// Whether the remote file size is estimated.
    estimated: bool,
}

/// Possible cache state of a package.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CacheMaybeValid {
    /// All files are cached and valid.
    Full,

    /// Some files are cached and valid.
    Partial,

    /// No valid cache.
    None,
}

/// Local cache information of a package.
struct PackageCache<'a> {
    /// Associated package.
    package: &'a Package,

    /// Whether the cache is valid.
    valid: CacheMaybeValid,

    /// Inner details of the package cache.
    ///
    /// Since a package may have multiple files to download, the inner hashmap
    /// stores the download information of each file.
    inner: HashMap<String, FileDownloadInfo<'a>>,
}

impl PackageCache<'_> {
    fn update_valid_state(&mut self) {
        let mut cnt = 0;
        for (_, cache) in self.inner.iter() {
            if cache.local_size == cache.remote_size {
                cnt += 1;
            }
        }

        if cnt == self.inner.len() {
            self.valid = CacheMaybeValid::Full;
        } else if cnt > 0 {
            self.valid = CacheMaybeValid::Partial;
        } else {
            self.valid = CacheMaybeValid::None;
        }
    }
}

impl<'a> PackageSet<'a> {
    pub fn new(
        session: &'a Session,
        packages: &'a [&Package],
        reuse_cache: bool,
    ) -> Fallible<PackageSet<'a>> {
        let mut multi = Multi::new();

        multi.set_max_host_connections(4)?;
        multi.pipelining(false, true)?;

        Ok(PackageSet {
            session,
            packages,
            multi,
            caches: LazyCell::new(),
            reuse_cache,
        })
    }

    fn load_cache(&self) {
        if self.caches.filled() {
            return;
        }

        let config = self.session.config();
        let cache_root = config.cache_path();

        let mut caches = HashMap::new();

        for &pkg in self.packages.iter() {
            // if the package is upgradable, use the upgradable reference instead
            let pkg = pkg.upgradable().unwrap_or(pkg);

            let urls = pkg.download_urls();
            let filenames = pkg.download_filenames();

            let mut pacakge_cache = PackageCache {
                package: pkg,
                valid: CacheMaybeValid::None,
                inner: HashMap::new(),
            };

            let mut file_cached_count = 0;
            for (url, filename) in urls.iter().zip(filenames.iter()) {
                let remote_size = 0u64;
                let mut local_size = 0u64;

                if self.reuse_cache {
                    if let Ok(file) = File::open(cache_root.join(filename)) {
                        if let Ok(metadata) = file.metadata() {
                            local_size = metadata.len();
                            file_cached_count += 1;
                        }
                    }
                }

                let dlinfo = FileDownloadInfo {
                    url,
                    local_size,
                    remote_size,
                    estimated: false,
                };

                pacakge_cache.inner.insert(filename.to_owned(), dlinfo);
            }

            if self.reuse_cache {
                if file_cached_count == urls.len() {
                    pacakge_cache.valid = CacheMaybeValid::Full;
                } else if file_cached_count > 0 {
                    pacakge_cache.valid = CacheMaybeValid::Partial;
                }
            }

            caches.insert(pkg.ident(), pacakge_cache);
        }

        let _ = self.caches.fill(caches);
    }

    /// Download packages.
    pub fn download(&self) -> Fallible<()> {
        if !self.caches.filled() {
            self.load_cache();
        }

        let config = self.session.config();
        let cache_root = config.cache_path();
        let proxy = config.proxy();
        let user_agent = self
            .session
            .user_agent
            .borrow()
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_USER_AGENT);

        let mut handles = HashMap::new();
        let mut token_ctx = HashMap::new();
        let package_caches = self.caches.borrow().unwrap();

        for (pidx, (_, cache)) in package_caches.iter().enumerate() {
            // skip download if all files are cached and valid
            if self.reuse_cache && cache.valid == CacheMaybeValid::Full {
                continue;
            }

            for (uidx, (filename, dlinfo)) in cache.inner.iter().enumerate() {
                if self.reuse_cache
                    && dlinfo.local_size > 0
                    && dlinfo.local_size == dlinfo.remote_size
                {
                    continue;
                }

                let mut easy = Easy::new();
                easy.get(true)?;
                easy.url(dlinfo.url)?;
                easy.follow_location(true)?;
                easy.useragent(user_agent)?;
                easy.fail_on_error(true)?;
                if let Some(proxy) = proxy {
                    easy.proxy(proxy)?;
                }

                if let Some(tx) = self.session.emitter() {
                    let ident = cache.package.ident();
                    let url = dlinfo.url.to_owned();
                    let fname = filename.to_owned();
                    easy.progress(true)?;
                    easy.progress_function(move |dltotal, dlnow, _, _| {
                        progress(
                            tx.clone(),
                            ident.to_owned(),
                            url.to_owned(),
                            fname.to_owned(),
                            dltotal,
                            dlnow,
                        )
                    })?;
                }

                let path = cache_root.join(filename);
                let tmp = path.join(".download");
                if path.exists() {
                    let _ = std::fs::remove_file(&path);
                }

                if tmp.exists() {
                    let _ = std::fs::remove_file(&tmp);
                }

                // TODO: Fragmented download support could be added to improve
                // download speed.
                let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
                easy.write_function(move |data| {
                    file.write_all(data).unwrap();
                    Ok(data.len())
                })?;

                let mut easyhandle = self.multi.add(easy)?;
                let token = pidx * 100 + uidx;
                let _ = easyhandle.set_token(token);
                handles.insert(token, easyhandle);

                token_ctx.insert(token, (cache.package.ident(), filename.to_owned()));
            }
        }

        let mut alive = true;
        while alive {
            alive = self.multi.perform()? > 0;

            let mut handle_err = None;

            self.multi.messages(|message| {
                let token = message.token().expect("failed to get token");
                let handle = handles.get_mut(&token).expect("failed to get handle");

                // catch and propagate curl error
                if let Some(Err(e)) = message.result_for(handle) {
                    handle_err = Some(e);
                }
            });

            if let Some(err) = handle_err {
                return Err(err.into());
            }

            if alive {
                self.multi.wait(&mut [], Duration::from_secs(5))?;
            }
        }

        Ok(())
    }

    /// Calculate download size.
    pub fn calculate_download_size(&mut self) -> Fallible<DownloadSize> {
        if !self.caches.filled() {
            self.load_cache();
        }

        let config = self.session.config();
        let proxy = config.proxy();
        let user_agent = self
            .session
            .user_agent
            .borrow()
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_USER_AGENT);

        let mut handles = HashMap::new();
        let mut token_ctx = HashMap::new();
        let package_caches = self.caches.borrow_mut().unwrap();

        for (pidx, &pkg) in self.packages.iter().enumerate() {
            // if the package is upgradable, use the upgradable reference instead
            let pkg = pkg.upgradable().unwrap_or(pkg);

            let urls = pkg.download_urls();
            let filenames = pkg.download_filenames();

            for (uidx, (url, filename)) in urls.iter().zip(filenames.iter()).enumerate() {
                let mut easy = Easy::new();
                easy.get(true)?;
                easy.url(url)?;
                easy.follow_location(true)?;
                easy.nobody(true)?;
                easy.useragent(user_agent)?;
                if let Some(proxy) = proxy {
                    easy.proxy(proxy)?;
                }

                let mut easyhandle = self.multi.add(easy)?;
                let token = pidx * 100 + uidx;
                let _ = easyhandle.set_token(token);
                handles.insert(token, easyhandle);

                token_ctx.insert(token, (pkg.ident(), url.to_string(), filename.to_owned()));
            }
        }

        let mut total = 0;
        let mut estimated = false;

        let mut alive = true;
        while alive {
            alive = self.multi.perform()? > 0;

            let mut handle_err = None;

            self.multi.messages(|message| {
                let token = message.token().expect("failed to get token");
                let handle = handles.get_mut(&token).expect("failed to get handle");

                if let Some(handle_ret) = message.result_for(handle) {
                    match handle_ret {
                        Err(e) => handle_err = Some(e),
                        Ok(_) => {
                            let (ident, url, filename) = token_ctx.get(&token).unwrap();
                            let package_cache = package_caches.get_mut(ident).unwrap();
                            let info = package_cache
                                .inner
                                .get_mut(filename)
                                .expect("failed to get cache info");

                            if let Ok(code) = handle.response_code() {
                                let mut content_length = 0u64;
                                if code == 200 {
                                    content_length =
                                        handle.content_length_download().unwrap_or(0f64) as u64;
                                    info.remote_size = content_length;
                                    if content_length != info.local_size {
                                        total += content_length;
                                    }
                                } else {
                                    debug!("code: {}, ident: {}, url: {}", code, ident, url)
                                }

                                if content_length == 0 {
                                    info.estimated = true;
                                    estimated = true;
                                }

                                package_cache.update_valid_state();
                            } else {
                                debug!("failed to get response code for {}", url);
                            }
                        }
                    }
                }
            });

            if let Some(err) = handle_err {
                return Err(err.into());
            }

            if alive {
                self.multi.wait(&mut [], Duration::from_secs(5))?;
            }
        }

        Ok(DownloadSize { total, estimated })
    }
}

/// Progress context for package download.
#[derive(Clone, Debug)]
pub struct PackageDownloadProgressContext {
    /// Package identifier.
    pub ident: String,

    /// Download URL.
    pub url: String,

    /// Download filename.
    pub filename: String,

    /// Total bytes to download.
    pub dltotal: u64,

    /// Downloaded bytes.
    pub dlnow: u64,
}

/// Report package download progress.
fn progress(
    tx: Sender<Event>,
    ident: String,
    url: String,
    filename: String,
    dltotal: f64,
    dlnow: f64,
) -> bool {
    let ctx = PackageDownloadProgressContext {
        ident,
        url,
        filename,
        dltotal: dltotal as u64,
        dlnow: dlnow as u64,
    };

    // TODO: progress threshold
    // it's not that efficient to send progress event to report every progress
    // change.
    tx.send(Event::PackageDownloadProgress(ctx)).is_ok()
}

// fn setup_working_dir(session: &Session, package: &Package) -> Fallible<(PathBuf, Vec<PathBuf>)> {
//     let files = package
//         .manifest
//         .url()
//         .into_iter()
//         .map(|url| {
//             session.config().cache_path.join(format!(
//                 "{}#{}#{}",
//                 package.name,
//                 package.version(),
//                 fs::filenamify(url)
//             ))
//         })
//         .collect::<Vec<_>>();

//     let version = match package.manifest.is_nightly() {
//         false => package.manifest.version().to_owned(),
//         true => {
//             let date = chrono::Local::now().format("%Y%m%d");
//             format!("nightly-{}", date)
//         }
//     };

//     let working_dir = session
//         .config()
//         .root_path
//         .join(format!("apps/{}/{}", package.name, version));
//     fs::ensure_dir(&working_dir)?;

//     for src in files.iter() {
//         let dst = working_dir.join(src.file_name().unwrap());
//         std::fs::copy(&src, &dst)?;
//     }

//     let ret = (working_dir, files);

//     // Return the last file as the fname
//     Ok(ret)
// }

// #[derive(Debug)]
// struct ChunkedRange {
//     pub offset: u64,
//     pub length: u64,
//     pub data: [u8; 4096],
// }

// fn download_packages<F>(
//     session: &Session,
//     packages: &Vec<Package>,
//     no_cache: bool,
//     callback: F,
// ) -> Fallible<()>
// where
//     F: FnMut(DownloadProgressContext) + Send + 'static,
// {
//     let callback = Arc::new(Mutex::new(callback));
//     let mut client = AgentBuilder::new();
//     let user_agent = match session.user_agent.filled() {
//         true => session.user_agent.borrow().unwrap().to_owned(),
//         false => constant::DEFAULT_USER_AGENT.to_string(),
//     };
//     client = client.user_agent(&user_agent);
//     let config = session.get_config();
//     if config.proxy().is_some() {
//         let proxy = config.proxy().unwrap();
//         let proxy = ureq::Proxy::new(proxy)?;
//         client = client.proxy(proxy);
//     }
//     let client = client.build();

//     for package in packages {
//         let urls = package.manifest.url();
//         let cookie = package.manifest.cookie();

//         let file_count = urls.len();

//         for (index, url) in urls.into_iter().enumerate() {
//             let index = index + 1;
//             let client = client.clone();
//             let cache_path = session.get_config().cache_path.join(format!(
//                 "{}#{}#{}",
//                 package.name,
//                 package.version(),
//                 fs::filenamify(url)
//             ));

//             if !no_cache && cache_path.exists() {
//                 continue;
//             }

//             // strip `#/dl.7z` url renaming
//             let url = url.split_once('#').map(|s| s.0).unwrap_or(url);

//             let mut request = client.get(url);

//             // Add cookie header if present
//             if let Some(cookie) = cookie {
//                 let mut cookies = vec![];
//                 for (key, value) in cookie {
//                     cookies.push(format!("{}={}", key, value));
//                 }
//                 let cookie = cookies.join("; ");
//                 request = request.set("Cookie", &cookie);
//             }

//             let response = request.call()?;

//             if response.status() != 200 {
//                 return Err(Error::Custom(format!(
//                     "failed to fetch {} (status code: {}",
//                     url,
//                     response.status()
//                 )));
//             }

//             let content_length = response
//                 .header("Content-Length")
//                 .map(|s| s.parse::<u64>().unwrap_or_default())
//                 .unwrap_or_default();
//             let accept_ranges = response
//                 .header("Accept-Ranges")
//                 .map(|s| "bytes" == s)
//                 .unwrap_or_default();

//             let cache_file = CacheFile::from(cache_path)?;

//             let ctx = DownloadProgressContext {
//                 name: package.ident(),
//                 total: content_length,
//                 position: 0,
//                 file_count,
//                 index,
//                 state: DownloadProgressState::Prepared,
//             };

//             let (tx, rx) = mpsc::channel::<ChunkedRange>();
//             let mut tasks = vec![];
//             if !accept_ranges {
//                 let pool = ThreadPool::builder().pool_size(2).create()?;

//                 let write_task = pool
//                     .spawn_with_handle(do_write(cache_file, ctx, rx, callback.clone()))
//                     .map_err(|e| Error::Custom(e.to_string()))?;
//                 tasks.push(write_task);

//                 let read_task = pool
//                     .spawn_with_handle(do_read(response, tx.clone()))
//                     .map_err(|e| Error::Custom(e.to_string()))?;
//                 tasks.push(read_task);
//             } else {
//                 let default_connections = 5;
//                 let split_size = 5_000_000 as u64;

//                 let x = content_length;
//                 let y = split_size;

//                 let split_count = (x / y + (x % y != 0) as u64) as usize;
//                 let connections = std::cmp::min(split_count, default_connections);

//                 let mut ranges = vec![];
//                 let mut range_start = 0;
//                 let mut range_end = 0;
//                 for _ in 1..=split_count {
//                     range_end += split_size;
//                     if range_end >= content_length {
//                         range_end = content_length - 1;
//                     }
//                     ranges.push((range_start, range_end));
//                     range_start = range_end + 1;
//                 }

//                 let pool_size = connections + 1;
//                 let pool = ThreadPool::builder().pool_size(pool_size).create()?;

//                 let write_task = pool
//                     .spawn_with_handle(do_write(cache_file, ctx, rx, callback.clone()))
//                     .map_err(|e| Error::Custom(e.to_string()))?;
//                 tasks.push(write_task);

//                 for range in ranges {
//                     let mut request = client.get(url);
//                     request = request.set("Range", &format!("bytes={}-{}", range.0, range.1));
//                     let read_task = pool
//                         .spawn_with_handle(do_read_range(request, range, tx.clone()))
//                         .map_err(|e| Error::Custom(e.to_string()))?;
//                     tasks.push(read_task);
//                 }
//             }
//             drop(tx);

//             let joined = futures::future::join_all(tasks);
//             futures::executor::block_on(joined);
//         }
//     }

//     Ok(())
// }

// async fn do_write<F>(
//     cache_file: CacheFile,
//     mut ctx: DownloadProgressContext,
//     rx: Receiver<ChunkedRange>,
//     callback: Arc<Mutex<F>>,
// ) -> Fallible<()>
// where
//     F: FnMut(DownloadProgressContext),
// {
//     let mut callback = callback.lock().unwrap();

//     let fd = std::fs::OpenOptions::new()
//         .truncate(true)
//         .create(true)
//         .write(true)
//         .open(cache_file.path())?;

//     // emit
//     callback(ctx.clone());

//     while let Ok(chunk) = rx.recv() {
//         let _ = fd.seek_write(&chunk.data[..chunk.length as usize], chunk.offset)?;

//         ctx.position = ctx.position + chunk.length;
//         if ctx.state != DownloadProgressState::Downloading {
//             ctx.state = DownloadProgressState::Downloading;
//         }
//         callback(ctx.clone());
//     }
//     drop(fd);

//     ctx.state = DownloadProgressState::Finished;
//     // emit
//     callback(ctx);
//     Ok(())
// }

// async fn do_read(response: ureq::Response, tx: Sender<ChunkedRange>) -> Fallible<()> {
//     let mut chunk = [0; 4096];
//     let mut offset = 0;
//     let mut reader = response.into_reader();

//     loop {
//         match reader.read(&mut chunk)? {
//             0 => break,
//             len => {
//                 let chunk = ChunkedRange {
//                     offset,
//                     length: len as u64,
//                     data: chunk,
//                 };
//                 offset += len as u64;
//                 tx.send(chunk).unwrap();
//             }
//         }
//     }
//     Ok(drop(tx))
// }

// async fn do_read_range(
//     request: Request,
//     range: (u64, u64),
//     tx: Sender<ChunkedRange>,
// ) -> Fallible<()> {
//     let response = request.call()?;
//     if !(response.status() >= 200 && response.status() <= 299) {
//         return Err(Error::Custom(format!(
//             "failed to fetch (status code: {})",
//             response.status()
//         )));
//     }

//     let mut chunk = [0; 4096];
//     let mut offset = range.0;
//     let mut reader = BufReader::new(response.into_reader());

//     loop {
//         match reader.read(&mut chunk)? {
//             0 => break,
//             length => {
//                 let chunked_range = ChunkedRange {
//                     offset,
//                     length: length as u64,
//                     data: chunk,
//                 };

//                 tx.send(chunked_range)
//                     .map_err(|e| Error::Custom(format!("failed to send chunk: {}", e)))?;

//                 offset += length as u64;
//             }
//         }
//     }
//     Ok(drop(tx))
// }

// pub(crate) fn verify_integrity(session: &Session, packages: &Vec<Package>) -> Fallible<()> {
//     println!("Verifying integrity of packages...");

//     for package in packages {
//         // skip nightly package
//         if package.manifest.is_nightly() {
//             continue;
//         }

//         let urls = package.manifest.url_with_hash();
//         print!("Checking hash of {}... ", package.name);

//         for (url, hash) in urls.into_iter() {
//             let cache_path = session.get_config().cache_path.join(format!(
//                 "{}#{}#{}",
//                 package.name,
//                 package.version(),
//                 fs::filenamify(url)
//             ));

//             let mut hasher = Checksum::new(hash).map_err(|e| Error::Custom(e.to_string()))?;
//             let mut file = std::fs::File::open(&cache_path)
//                 .with_context(|| format!("failed to open cache file: {}", cache_path.display()))?;
//             let mut buffer = [0; 4096];
//             loop {
//                 let len = file.read(&mut buffer).with_context(|| {
//                     format!("failed to read cache file: {}", cache_path.display())
//                 })?;
//                 match len {
//                     0 => break,
//                     len => hasher.consume(&buffer[..len]),
//                 }
//             }
//             let checksum = hasher.result();
//             if hash != &checksum {
//                 println!("Err");
//                 return Err(Error::Custom(format!(
//                     "checksum mismatch: {}\n Expected: {}\n Actual: {}",
//                     cache_path.display(),
//                     hash,
//                     checksum
//                 )));
//             }
//         }
//         println!("Ok");
//     }

//     Ok(())
// }
