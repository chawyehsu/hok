use std::cmp::min;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use once_cell::sync::Lazy;
use regex::Regex;
use regex::RegexBuilder;
use scoop_hash::Checksum;
use tokio_stream::StreamExt;

use crate::indicator::pb_download;
use crate::manager::CacheManager;
use crate::model::AvailableApp;
use crate::model::CacheFile;
use crate::model::Hash;
use crate::ops::app::search_available_app;
use crate::Config;
use crate::DepGraph;
use crate::ScoopResult;
use crate::HttpClient;
use crate::util::ensure_dir;
use crate::util::remove_dir;

pub fn resolve_install_order(config: &Config, apps: Vec<&str>) -> ScoopResult<Vec<String>> {
    let mut graph = DepGraph::<String>::new();
    for app in apps.into_iter() {
        visit(config, &mut graph, app)?;
    }
    log::debug!("{:?}", graph);
    let apps = graph.flat_walk()?;
    return Ok(apps);

    fn visit(config: &Config, graph: &mut DepGraph<String>, pattern: &str) -> ScoopResult<()> {
        let app = search_available_app(config, pattern)?;
        let deps = app.manifest().get_deps();
        graph.register_deps(pattern.to_owned(), deps.clone());
        graph.check()?;
        for dep in deps.into_iter() {
            visit(config, graph, &dep)?;
        }
        Ok(())
    }
}

fn install_failed_callback(config: &Config, app: &AvailableApp) -> ScoopResult<()> {
    let install_path = config.apps_path().join(app.name());
    Ok(remove_dir(&install_path)?)
}

pub async fn install<'a>(config: &Config, app: &AvailableApp<'a>,
    ignore_cache: bool,
    skip_hash_validation: bool,
) -> ScoopResult<()> {
    match workflow_exec(config, app, ignore_cache, skip_hash_validation).await {
        Ok(code) => match code {
            true => Ok(()),
            false => {
                install_failed_callback(config, app)?;
                anyhow::bail!("Install failed");
            }
        },
        Err(e) => {
            install_failed_callback(config, app)?;
            Err(e)
        }
    }
}

async fn workflow_exec<'a>(config: &Config, app: &AvailableApp<'a>,
    ignore_cache: bool,
    skip_hash_validation: bool,
) -> ScoopResult<bool> {
    let cache_files = download_files(config, app, ignore_cache, skip_hash_validation).await?;

    let install_path = config.apps_path().join(app.name()).join(app.manifest().version());
    ensure_dir(&install_path)?;

    let extract_dir = app.manifest().get_extract_dir();
    let extract_to = app.manifest().get_extract_to();

    log::debug!("{:?} {:?}", extract_dir, extract_to);
    static RE: Lazy<Regex> = Lazy::new(|| {
        let p = r"\.((gz)|(tar)|(tgz)|(lzma)|(bz)|(bz2)|(7z)|(rar)|(iso)|(xz)|(lzh)|(nupkg))$";
        RegexBuilder::new(p).build().unwrap()
    });

    for cache_file in cache_files.into_iter() {
        let dest = PathBuf::from(install_path.clone()).join(cache_file.file_name());
        std::fs::copy(cache_file.path(), dest.as_path())?;

        let extract_fn = if app.manifest().is_innosetup() {
            Some("Expand-InnoArchive")
        } else if cache_file.file_name().as_str().ends_with(".zip") {
            Some("Expand-ZipArchive")
        } else if cache_file.file_name().as_str().ends_with(".msi") {
            Some("Expand-MsiArchive")
        } else {
            if RE.is_match(cache_file.file_name().as_str()) {
                Some("Expand-7zipArchive")
            } else {
                None
            }
        };

        if extract_fn.is_some() {
            println!("Extracting {} ...", cache_file.file_name());
            // std::io::stdout().flush()?;
            let mut cmd = std::process::Command::new("pwsh");
            cmd.arg("-NoProfile");
            cmd.arg("-Command");
            // cmd.arg(format!("& Write-Host -f Cyan \"Hello PowerShell\""));
            cmd.arg(format!("& {} -Path \"{}\" -DestinationPath \"{}\"",
                extract_fn.unwrap(),
                dest.as_path().display(),
                install_path.as_path().display(),
            ));
            return Ok(cmd.spawn()?.wait()?.success());
        }
    }

    Ok(true)
}

fn validate_hash(cache_file: &CacheFile, hash: &Hash) -> ScoopResult<()> {
    let mut checksum = Checksum::new(hash);
    let mut data = Vec::new();
    File::open(cache_file.path())?.read_to_end(&mut data)?;
    checksum.consume(&data);
    match checksum.checksum() {
        true => Ok(()),
        false => anyhow::bail!("Hash check failed!\n"),
    }
}

async fn download_files<'a>(
    config: &Config,
    app: &AvailableApp<'a>,
    ignore_cache: bool,
    skip_hash_validation: bool,
) -> ScoopResult<Vec<CacheFile>> {
    let http_client = HttpClient::new(config).unwrap();
    let cache_manager = CacheManager::new(config);
    let urls = app.manifest().get_url();
    let hashes = app.manifest().get_hash();
    let mut cache_files = Vec::new();

    if app.manifest().version() != "nightly" {
        if hashes.is_none() {
            anyhow::bail!("No hash found for {}, download stopped", app.name());
        }

        if urls.len() != hashes.unwrap().len() {
            anyhow::bail!("Url&Hash count mismatch for {}, download stopped", app.name());
        }
    } else if app.manifest().version() == "nightly" {
        eprintln!("Warning: installing nightly version, hash validation disabled");
    } else if skip_hash_validation {
        eprintln!("Warning: hash validation explicitly disabled");
    }

    if app.manifest().version() == "nightly" || skip_hash_validation {
        for url in urls.iter() {
            // Remove the trailing file renaming part
            let actual_url = url.split_once("#/").unwrap_or((url, "")).0;
            println!("==> Downloading {}", actual_url);

            // Prepare the CacheFile
            let cache_file = cache_manager.add(app.name(), app.manifest().version(), url);
            if !ignore_cache && cache_file.path().exists() {
                println!("Already downloaded: {}", cache_file.path().display());
            } else {
                // Http call
                let resp = http_client.get(actual_url).send().await.unwrap();
                if resp.status().is_success() || resp.status().is_redirection() {
                    let mut temp_file = File::create(cache_file.tmp_path()).unwrap();
                    let mut downloaded = 0u64;
                    let len = resp.content_length().unwrap();
                    let pb = pb_download(len);

                    let mut stream = resp.bytes_stream();
                    // Write stream data into the tmp file and display the progress
                    while let Some(item) = stream.next().await {
                        let chunk = item.unwrap();
                        temp_file.write(&chunk).unwrap();
                        let new = min(downloaded + (chunk.len() as u64), len);
                        downloaded = new;
                        pb.set_position(new);
                    }
                    // Move the temp cache to the real cache
                    std::fs::rename(cache_file.tmp_path(), cache_file.path()).unwrap();
                    // Finalize the progress
                    pb.finish();
                    println!("✓ Downloaded: {}", cache_file.path().display());
                } else {
                    anyhow::bail!("Failed to download {}: {}", actual_url, resp.status());
                }
            }

            cache_files.push(cache_file);
        }
    } else {
        for (url, hash) in urls.iter().zip(hashes.unwrap().iter()) {
            // Remove the trailing file renaming part
            let actual_url = url.split_once("#/").unwrap_or((url, "")).0;
            println!("==> Downloading {}", actual_url);

            // Prepare the CacheFile
            let cache_file = cache_manager.add(app.name(), app.manifest().version(), url);
            if !ignore_cache && cache_file.path().exists() {
                println!("Already downloaded: {}", cache_file.path().display());
            } else {
                // Http call
                let resp = http_client.get(actual_url).send().await.unwrap();
                if resp.status().is_success() || resp.status().is_redirection() {
                    let mut temp_file = File::create(cache_file.tmp_path()).unwrap();
                    let mut downloaded = 0u64;
                    let len = resp.content_length().unwrap();
                    let pb = pb_download(len);

                    let mut stream = resp.bytes_stream();
                    // Write stream data into the tmp file and display the progress
                    while let Some(item) = stream.next().await {
                        let chunk = item.unwrap();
                        temp_file.write(&chunk).unwrap();
                        let new = min(downloaded + (chunk.len() as u64), len);
                        downloaded = new;
                        pb.set_position(new);
                    }
                    // Move the temp cache to the real cache
                    std::fs::rename(cache_file.tmp_path(), cache_file.path()).unwrap();
                    // Finalize the progress
                    pb.finish();
                    println!("✓ Downloaded: {}", cache_file.path().display());
                } else {
                    anyhow::bail!("Failed to download {}: {}", actual_url, resp.status());
                }
            }

            // Validate the hash
            if !skip_hash_validation {
                print!("Checking hash...");
                std::io::stdout().flush().unwrap();
                validate_hash(&cache_file, hash)?;
                println!(" ok");
            }

            cache_files.push(cache_file);
        }
    }

    Ok(cache_files)
}
