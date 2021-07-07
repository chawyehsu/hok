use crate::indicator::pb_download;
use crate::tokio_util::{block_on, StreamExt};
use clap::ArgMatches;
use scoop_core::fs::leaf;
use scoop_core::{find_manifest, AppManager, CacheManager, Config, HttpClient};
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

async fn download_file(
    config: &Config,
    app: &str,
    version: &str,
    url: &str,
    ignore_cache: bool,
    hash: Option<&str>,
) {
    // Remove the trailing file renaming part
    let original_url = url.split_once("#").unwrap_or((url, "")).0;
    let original_fname = leaf(&PathBuf::from(original_url));

    log::debug!("{} original download url: {}", app, original_url);

    // Prepare the CacheEntry
    let cache_manager = CacheManager::new(config);
    let ce = cache_manager.add(app, version, url);
    let cache_real_path = ce.path();

    if !ignore_cache && cache_real_path.exists() {
        println!("{} exists, skip download.", original_fname);
        return;
    }

    let cache_temp_path = ce.tmp_path();
    log::debug!("{} cache real path: {}", app, cache_real_path.display());
    log::debug!("{} cache temp path: {}", app, cache_temp_path.display());

    // Send http request to get the file
    let http_client = HttpClient::new(config).unwrap();
    let resp = http_client.get(original_url).send().await.unwrap();
    assert!(resp.status().is_success());

    // Create cache tmp file
    let mut cache_file = File::create(cache_temp_path.as_path()).unwrap();
    let mut downloaded = 0u64;
    let dl_size = resp.content_length().unwrap();
    let pb = pb_download(original_fname.as_str(), dl_size);

    // Init Checksum
    let mut checksum = None;
    if hash.is_some() {
        checksum = Some(scoop_hash::Checksum::new(hash.unwrap()));
    }

    let mut stream = resp.bytes_stream();
    // Write stream data into the tmp file and display the progress
    while let Some(item) = stream.next().await {
        let chunk = item.unwrap();
        cache_file.write(&chunk).unwrap();

        // Checksum consume data
        if hash.is_some() {
            checksum.as_mut().unwrap().consume(&chunk);
        }

        let new = min(downloaded + (chunk.len() as u64), dl_size);
        downloaded = new;
        pb.set_position(new);
    }
    // Move the temp cache to the real cache
    std::fs::rename(cache_temp_path, cache_real_path).unwrap();
    // Finalize the progress
    pb.finish();

    // Finalize the checksum
    let hash = hash
        .unwrap()
        .split_once(":")
        .unwrap_or(("", hash.unwrap()))
        .1;
    let actual_checksum = checksum.unwrap().result();
    if hash != actual_checksum {
        eprintln!(
            "Checking hash of {} ... ERROR Hash check failed!",
            original_fname
        );
        eprintln!("URL:      {}", url);
        eprintln!("Expected: {}", hash);
        eprintln!("Actual:   {}", actual_checksum);
        eprintln!();
    }
}

pub fn cmd_install(matches: &ArgMatches, config: &Config) {
    if let Some(apps) = matches.values_of("app") {
        let ignore_cache = matches.is_present("ignore_cache");
        let skip_hash_validation = matches.is_present("skip_hash_validation");

        let app_manager = AppManager::new(config);
        let mut manifests = Vec::new();
        for app in apps.into_iter() {
            let (_, app_name) = match app.contains("/") {
                true => app.split_once("/").unwrap(),
                false => ("", app),
            };

            if app_manager.is_app_installed(app_name) {
                eprintln!("'{}' is already installed.", app_name);
                std::process::exit(1);
            }

            match find_manifest(config, app).unwrap() {
                Some(man) => manifests.push((app_name, man)),
                None => {
                    eprintln!("Couldn't find manifest for '{}'", app);
                    std::process::exit(1);
                }
            }
        }

        manifests.iter().for_each(|(app, manifest)| {
            let version = manifest.get_version();
            let urls = manifest.get_url();

            if version == "nightly" || skip_hash_validation {
                urls.iter().for_each(|url| {
                    block_on(download_file(config, app, version, url, ignore_cache, None));
                })
            } else {
                let hashes = manifest.get_hash();
                if hashes.is_none() {
                    eprint!(
                        "no hash is provided in '{}''s manifest, skipped download.",
                        app
                    );
                    return;
                }

                let hashes = hashes.unwrap();

                // number of hashes needs to be equal to number of urls
                if hashes.len() != urls.len() {
                    eprintln!("missing hashes in '{}''s manifest, skipped download.", app);
                }

                urls.iter().zip(hashes.iter()).for_each(|(url, hash)| {
                    log::debug!("url {}, hash {:?}", url, hash);
                    block_on(download_file(
                        config,
                        app,
                        version,
                        url,
                        ignore_cache,
                        Some(&**hash),
                    ));
                })
            }
        });
    }
}
