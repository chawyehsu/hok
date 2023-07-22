#![allow(unused)]
use std::{
    collections::HashSet,
    io::{BufReader, Read},
    os::windows::prelude::FileExt,
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
};

use futures::{executor::ThreadPool, task::SpawnExt};
use ureq::{AgentBuilder, Request};

use crate::{
    cache::CacheFile,
    constant,
    error::{Error, Fallible},
    internal::fs,
    Session,
};

use super::{InstallOption, Package};

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

pub(crate) fn install_packages(
    session: &Session,
    packages: Vec<Package>,
    options: HashSet<InstallOption>,
) -> Fallible<()> {
    let no_cache = options.contains(&InstallOption::IgnoreCache);
    // let ignore_failure = options.contains(&InstallOption::IgnoreFailure);

    // download
    // download_packages(session, &packages, no_cache)?;
    let download_only = options.contains(&InstallOption::DownloadOnly);
    if download_only {
        return Ok(());
    }

    // verrify integrity
    let no_hash_check = options.contains(&InstallOption::NoHashCheck);
    if !no_hash_check {
        // verify_integrity(session, &packages)?;
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
            session.get_config().cache_path.join(format!(
                "{}#{}#{}",
                package.name,
                package.version(),
                fs::filenamify(url)
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
        .get_config()
        .root_path
        .join(format!("apps/{}/{}", package.name, version));
    fs::ensure_dir(&working_dir)?;

    for src in files.iter() {
        let dst = working_dir.join(src.file_name().unwrap());
        std::fs::copy(&src, &dst)?;
    }

    let ret = (working_dir, files);

    // Return the last file as the fname
    Ok(ret)
}

#[derive(Debug)]
struct ChunkedRange {
    pub offset: u64,
    pub length: u64,
    pub data: [u8; 4096],
}

fn download_packages<F>(
    session: &Session,
    packages: &Vec<Package>,
    no_cache: bool,
    callback: F,
) -> Fallible<()>
where
    F: FnMut(DownloadProgressContext) + Send + 'static,
{
    let callback = Arc::new(Mutex::new(callback));
    let mut client = AgentBuilder::new();
    let user_agent = match session.user_agent.filled() {
        true => session.user_agent.borrow().unwrap().to_owned(),
        false => constant::DEFAULT_USER_AGENT.to_string(),
    };
    client = client.user_agent(&user_agent);
    let config = session.get_config();
    if config.proxy().is_some() {
        let proxy = config.proxy().unwrap();
        let proxy = ureq::Proxy::new(proxy)?;
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
            let cache_path = session.get_config().cache_path.join(format!(
                "{}#{}#{}",
                package.name,
                package.version(),
                fs::filenamify(url)
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

            let response = request.call()?;

            if response.status() != 200 {
                return Err(Error::Custom(format!(
                    "failed to fetch {} (status code: {}",
                    url,
                    response.status()
                )));
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
                let pool = ThreadPool::builder().pool_size(2).create()?;

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
                let pool = ThreadPool::builder().pool_size(pool_size).create()?;

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
        .open(cache_file.path())?;

    // emit
    callback(ctx.clone());

    while let Ok(chunk) = rx.recv() {
        let _ = fd.seek_write(&chunk.data[..chunk.length as usize], chunk.offset)?;

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
        match reader.read(&mut chunk)? {
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
    let response = request.call()?;
    if !(response.status() >= 200 && response.status() <= 299) {
        return Err(Error::Custom(format!(
            "failed to fetch (status code: {})",
            response.status()
        )));
    }

    let mut chunk = [0; 4096];
    let mut offset = range.0;
    let mut reader = BufReader::new(response.into_reader());

    loop {
        match reader.read(&mut chunk)? {
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
