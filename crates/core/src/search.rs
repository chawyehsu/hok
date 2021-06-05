use std::path::Path;

use anyhow::Result;
use futures::{executor::block_on, future::join_all};
// use log::trace;
use crate::{bucket::Bucket, fs::leaf_base, manifest::Manifest, Scoop};
use serde_json::Value;

struct SearchMatch {
    name: String,
    version: String,
    bin: Option<String>,
}

struct Matches {
    bucket: String,
    collected: Vec<SearchMatch>,
}

async fn walk_manifests(
    bucket_name: &str,
    bucket: &Bucket,
    query: &str,
    with_binary: bool,
) -> Result<Matches> {
    let mut search_matches: Vec<SearchMatch> = Vec::new();

    for app in bucket.available_manifests()?.iter() {
        // trace!("Searching manifest {}", app.display());

        let app_name = leaf_base(app);

        // substring check on app_name
        if app_name.contains(query) {
            let manifest = Manifest::from_path(app, Some(bucket_name.to_string()));
            if manifest.is_err() {
                continue;
            }
            let manifest = manifest?;
            let version = manifest.version;
            let name = app_name.to_string();
            let bin: Option<String> = None;

            let sm = SearchMatch { name, version, bin };

            search_matches.push(sm);
        } else {
            // Searching binaries requires a very-high overhead (reading all json files),
            // will not do binary search without the option.
            if !with_binary {
                continue;
            }

            let manifest = Manifest::from_path(app, Some(bucket_name.to_string()));
            if manifest.is_err() {
                continue;
            }
            let manifest = manifest?;
            let bin = manifest.json.get("bin");

            // filter manifest doesn't contain `bin`
            if bin.is_none() {
                continue;
            }

            let bin = bin.unwrap();
            let match_bin: Option<Vec<String>> = match bin {
                Value::String(bin) => {
                    let bin = Path::new(bin).file_name().unwrap().to_str().unwrap();

                    if bin.contains(query) {
                        Some(vec![format!("'{}'", bin.to_string())])
                    } else {
                        None
                    }
                }
                Value::Array(bins) => {
                    let mut bin_matches = Vec::new();

                    for bin in bins {
                        match bin {
                            Value::String(bin) => {
                                let bin = Path::new(bin).file_name().unwrap().to_str().unwrap();

                                if bin.contains(query) {
                                    bin_matches.push(format!("'{}'", bin.to_string()));
                                    continue;
                                }
                            }
                            Value::Array(bin_pair) => {
                                // test bin
                                let bin = bin_pair.get(0).unwrap();
                                match bin {
                                    Value::String(bin) => {
                                        let bin =
                                            Path::new(bin).file_name().unwrap().to_str().unwrap();

                                        if bin.contains(query) {
                                            bin_matches.push(format!("'{}'", bin.to_string()));
                                            continue;
                                        }
                                    }
                                    _ => {}
                                }

                                // test alias
                                let bin = bin_pair.get(1).unwrap();
                                match bin {
                                    Value::String(bin) => {
                                        if bin.contains(query) {
                                            bin_matches.push(format!("'{}'", bin.to_string()));
                                            continue;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }

                    if bin_matches.len() > 0 {
                        Some(bin_matches)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            match match_bin {
                Some(bins) => {
                    let version = manifest.json.get("version");
                    let name = app_name.to_string();
                    let version = version.unwrap().as_str().unwrap().to_owned();
                    let bin: Option<String> = Some(bins.get(0).unwrap().to_owned());

                    let sm = SearchMatch { name, version, bin };

                    search_matches.push(sm);
                }
                None => {
                    continue;
                }
            }
        }
    }

    Ok(Matches {
        bucket: bucket_name.to_string(),
        collected: search_matches,
    })
}

impl<'a> Scoop<'a> {
    pub fn search(&mut self, query: &str, with_binary: bool) -> Result<()> {
        // Load all local buckets
        let buckets = self.bucket_manager.get_buckets();

        let mut matches: Vec<Matches> = Vec::new();
        let mut futures = Vec::new();

        for (bucket_name, bucket) in buckets {
            futures.push(walk_manifests(bucket_name, bucket, query, with_binary));
        }

        block_on(async {
            let all_matches = join_all(futures).await;
            for m in all_matches {
                matches.push(m.unwrap());
            }
        });

        matches.sort_by_key(|k| k.bucket.to_string());

        for m in matches {
            if m.collected.len() > 0 {
                println!("'{}' bucket:", m.bucket);
                for sm in m.collected {
                    if sm.bin.is_none() {
                        println!("  {} ({})", sm.name, sm.version);
                    } else {
                        println!(
                            "  {} ({}) --> includes {}",
                            sm.name,
                            sm.version,
                            sm.bin.unwrap()
                        );
                    }
                }
                println!("");
            }
        }

        Ok(())
    }
}
