use crate::indicator::pb_download;
use crate::tokio_util::{block_on, StreamExt};
use clap::ArgMatches;
use scoop_core::Scoop;
use scoop_core::fs::leaf;
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

async fn download_file<'a>(scoop: &Scoop<'a>, app: &str, version: &str, url: &str) {
    let download_url = url.split_once("#").unwrap_or((url, "")).0;
    log::debug!("{} (download url): {}", app, download_url);

    let resp = scoop.http.get(download_url).send().await.unwrap();
    assert!(resp.status().is_success());

    let filename = leaf(&PathBuf::from(download_url));
    let total_size = resp.content_length().unwrap();
    let escaped = crate::utils::escape_filename(url);
    let pb = pb_download(filename.as_str(), total_size);
    let cache_to_name = format!("{}#{}#{}", app, version, escaped);
    let saved_name = scoop.cache_manager.create(cache_to_name);
    log::debug!("{} (cache path): {}", app, saved_name.display());

    let mut tmp_file = saved_name.clone().into_os_string();
    tmp_file.push(".download");
    let tmp_file = PathBuf::from(tmp_file);
    log::debug!("{} (tmp file): {}", app, tmp_file.display());

    let mut cache_file = File::create(tmp_file.as_path()).unwrap();
    let mut downloaded = 0u64;
    let mut stream = resp.bytes_stream();


    while let Some(item) = stream.next().await {
        let chunk = item.unwrap();
        cache_file.write(&chunk).unwrap();
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    std::fs::rename(tmp_file, saved_name).unwrap();
    pb.finish();
}

pub fn cmd_install(matches: &ArgMatches, scoop: &mut Scoop) {
    if let Some(apps) = matches.values_of("app") {
        let mut manifests = Vec::new();
        for app in apps.into_iter() {
            let (_, app_name) = match app.contains("/") {
                true => app.split_once("/").unwrap(),
                false => ("", app),
            };

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
                    block_on(download_file(scoop, app, version, url));
                })
            }
        });
    }
}
