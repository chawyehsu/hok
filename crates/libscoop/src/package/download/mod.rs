//! Downloading a package's files into the cache.
//!
//! Three pieces share this module so the download vocabulary has one home:
//!
//! - [`Download`]: one URL of a manifest, with the name it installs under.
//! - [`cache`]: the cache file name and its inverse.
//! - [`PackageSet`]: the cached files of a set of packages, and the transfer
//!   that fills them.

mod cache;
mod entry;

#[cfg(test)]
mod tests;

pub(crate) use cache::{CacheFileName, cache_file_name};
pub(crate) use entry::Download;

use curl::easy::{Easy, List};
use curl::multi::Multi;
use flume::Sender;
use once_cell::unsync::OnceCell;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Write,
    time::Duration,
};
use tracing::debug;

use crate::constant::DEFAULT_USER_AGENT;
use crate::{Event, Session, error::Fallible, internal};

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
    pub packages: &'a [&'a Package],

    /// Multi handle for curl.
    multi: Multi,

    caches: OnceCell<HashMap<String, PackageCache<'a>>>,

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

        // TODO: configurable max connections
        multi.set_max_total_connections(6)?;
        multi.set_max_host_connections(4)?;
        multi.pipelining(false, true)?;

        Ok(PackageSet {
            session,
            packages,
            multi,
            caches: OnceCell::new(),
            reuse_cache,
        })
    }

    fn load_cache(&self) {
        if self.caches.get().is_some() {
            return;
        }

        let config = self.session.config();
        let cache_root = config.cache_path();

        let mut caches = HashMap::new();

        for &pkg in self.packages.iter() {
            // if the package is upgradable, use the upgradable reference instead
            let pkg = pkg.upgradable().unwrap_or(pkg);

            let downloads = pkg.downloads();

            let mut pacakge_cache = PackageCache {
                package: pkg,
                valid: CacheMaybeValid::None,
                inner: HashMap::new(),
            };

            let mut file_cached_count = 0;
            for download in downloads.iter() {
                let filename = cache_file_name(pkg.name(), pkg.version(), download);
                let remote_size = 0u64;
                let mut local_size = 0u64;

                if self.reuse_cache
                    && let Ok(file) = File::open(cache_root.join(&filename))
                    && let Ok(metadata) = file.metadata()
                {
                    local_size = metadata.len();
                    file_cached_count += 1;
                }

                let dlinfo = FileDownloadInfo {
                    url: download.url,
                    local_size,
                    remote_size,
                    estimated: false,
                };

                pacakge_cache.inner.insert(filename, dlinfo);
            }

            if self.reuse_cache {
                if file_cached_count == downloads.len() {
                    pacakge_cache.valid = CacheMaybeValid::Full;
                } else if file_cached_count > 0 {
                    pacakge_cache.valid = CacheMaybeValid::Partial;
                }
            }

            caches.insert(pkg.ident(), pacakge_cache);
        }

        let _ = self.caches.set(caches);
    }

    /// Download packages.
    pub fn download(&mut self) -> Fallible<()> {
        if self.caches.get().is_none() {
            self.load_cache();
        }

        let config = self.session.config();
        let cache_root = config.cache_path();
        let proxy = config.proxy();
        let user_agent = self
            .session
            .user_agent
            .get()
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_USER_AGENT);

        let mut handles = HashMap::new();
        let mut token_ctx = HashMap::new();
        let package_caches = self.caches.get_mut().unwrap();

        // map download tmp files to their final names
        let mut filepaths = vec![];

        // ensure cache dir exists
        internal::fs::ensure_dir(&cache_root)?;

        for (pidx, (_, cache)) in package_caches.iter().enumerate() {
            // skip download if all files are cached and valid
            if self.reuse_cache && cache.valid == CacheMaybeValid::Full {
                continue;
            }

            let cookie = cache.package.cookie().unwrap_or_default();

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
                set_cookie(&mut easy, &cookie)?;

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
                let tmp = cache_root.join(format!("{}.download", filename));

                // remove possible existing files
                let _ = std::fs::remove_file(&path);
                let _ = std::fs::remove_file(&tmp);

                filepaths.push((tmp.clone(), path.clone()));

                // TODO: Fragmented download support could be added to improve
                // download speed.
                let mut file = OpenOptions::new().create(true).append(true).open(&tmp)?;
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

        for (tmp, path) in filepaths.iter() {
            std::fs::rename(tmp, path)?;
        }

        Ok(())
    }

    /// Calculate download size.
    ///
    /// This function is actually a pre-download process, which will try to
    /// fetch the remote file size of each package file.
    pub fn calculate_download_size(&mut self) -> Fallible<DownloadSize> {
        if self.caches.get().is_none() {
            self.load_cache();
        }

        let config = self.session.config();
        let proxy = config.proxy();
        let user_agent = self
            .session
            .user_agent
            .get()
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_USER_AGENT);

        let mut handles = HashMap::new();
        let mut token_ctx = HashMap::new();
        let package_caches = self.caches.get_mut().unwrap();

        for (pidx, &pkg) in self.packages.iter().enumerate() {
            // if the package is upgradable, use the upgradable reference instead
            let pkg = pkg.upgradable().unwrap_or(pkg);

            let downloads = pkg.downloads();
            let cookie = pkg.cookie().unwrap_or_default();

            for (uidx, download) in downloads.iter().enumerate() {
                let url = download.url;
                let filename = cache_file_name(pkg.name(), pkg.version(), download);
                let mut easy = Easy::new();
                easy.get(true)?;
                easy.url(url)?;
                easy.follow_location(true)?;
                easy.nobody(true)?;
                easy.useragent(user_agent)?;
                if let Some(proxy) = proxy {
                    easy.proxy(proxy)?;
                }
                set_cookie(&mut easy, &cookie)?;

                let mut easyhandle = self.multi.add(easy)?;
                let token = pidx * 100 + uidx;
                let _ = easyhandle.set_token(token);
                handles.insert(token, easyhandle);

                token_ctx.insert(token, (pkg.ident(), url.to_string(), filename));
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

fn set_cookie(easy: &mut Easy, cookie: &[(&str, &str)]) -> Fallible<()> {
    if !cookie.is_empty() {
        let mut header_cookie = String::from("Cookie: ");
        header_cookie.push_str(
            &cookie
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; "),
        );
        let mut list = List::new();
        list.append(&header_cookie)?;
        easy.http_headers(list)?;
    }

    Ok(())
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
