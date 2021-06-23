use crate::indicator::pb_download;
use crate::tokio_util::{block_on, StreamExt};
use clap::ArgMatches;
use scoop_core::fs::leaf;
use scoop_core::Scoop;
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

async fn download_file<'a>(
    scoop: &Scoop<'a>,
    app: &str,
    version: &str,
    url: &str,
    ignore_cache: bool,
) {
    // Remove the trailing file renaming part
    let original_url = url.split_once("#").unwrap_or((url, "")).0;
    let original_fname = leaf(&PathBuf::from(original_url));

    log::debug!("{} original download url: {}", app, original_url);

    // Prepare the CacheEntry
    let ce = scoop.cache_manager.add(app, version, url);
    let cache_real_path = ce.path();

    if !ignore_cache && cache_real_path.exists() {
        println!("{} exists, skip download.", original_fname);
        return;
    }

    let cache_temp_path = ce.tmp_path();
    log::debug!("{} cache real path: {}", app, cache_real_path.display());
    log::debug!("{} cache temp path: {}", app, cache_temp_path.display());

    // Send http request to get the file
    let resp = scoop.http.get(original_url).send().await.unwrap();
    assert!(resp.status().is_success());

    // Create cache tmp file
    let mut cache_file = File::create(cache_temp_path.as_path()).unwrap();
    let mut downloaded = 0u64;
    let dl_size = resp.content_length().unwrap();
    let pb = pb_download(original_fname.as_str(), dl_size);

    let mut stream = resp.bytes_stream();
    // Write stream data into the tmp file and display the progress
    while let Some(item) = stream.next().await {
        let chunk = item.unwrap();
        cache_file.write(&chunk).unwrap();
        let new = min(downloaded + (chunk.len() as u64), dl_size);
        downloaded = new;
        pb.set_position(new);
    }
    // Move the temp cache to the real cache
    std::fs::rename(cache_temp_path, cache_real_path).unwrap();
    // Finalize the progress
    pb.finish();

    // TODO: Check hash
}

pub fn cmd_install(matches: &ArgMatches, scoop: &mut Scoop) {
    if let Some(apps) = matches.values_of("app") {
        let ignore_cache = matches.is_present("ignore_cache");

        let mut manifests = Vec::new();
        for app in apps.into_iter() {
            let (_, app_name) = match app.contains("/") {
                true => app.split_once("/").unwrap(),
                false => ("", app),
            };

            if scoop.app_manager.is_app_installed(app_name) {
                eprintln!("'{}' is already installed.", app_name);
                std::process::exit(1);
            }

            match scoop.find_local_manifest(app).unwrap() {
                Some(man) => manifests.push((app_name, man)),
                None => {
                    eprintln!("Couldn't find manifest for '{}'", app);
                    std::process::exit(1);
                }
            }
        }

        manifests.iter().for_each(|(app, manifest)| {
            let version = &manifest.data.version;
            let urls = manifest.get_download_urls();
            // let hashes = manifest.get_hashes();

            if urls.is_some() {
                let urls = urls.unwrap();

                urls.iter().for_each(|url| {
                    block_on(download_file(scoop, app, version, url, ignore_cache));
                })
            }
        });
    }
}
