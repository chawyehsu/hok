use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    fs::leaf_base,
    manifest::{BinType, Manifest, StringOrStringArray},
    Scoop,
};
use anyhow::Result;
use rayon::prelude::*;

#[derive(Clone, Debug)]
struct SearchMatch {
    name: String,
    version: String,
    bin: Option<String>,
}

struct Matches {
    bucket: String,
    collected: Vec<SearchMatch>,
}

fn try_match_bin(query: &str, input: Option<BinType>) -> Vec<String> {
    let mut bin_matches = Vec::new();
    match input {
        None => {}
        Some(bintype) => match bintype {
            BinType::String(bin) => {
                if bin.contains(query) {
                    bin_matches.push(bin);
                }
            }
            BinType::Array(arr) => {
                arr.into_iter().for_each(|item| match item {
                    StringOrStringArray::String(bin) => {
                        if bin.contains(query) {
                            bin_matches.push(bin);
                        }
                    }
                    StringOrStringArray::Array(pair) => {
                        if pair[1].contains(query) {
                            bin_matches.push(pair[1].to_string());
                        }
                    }
                });
            }
        },
    }

    bin_matches
}

fn travel_manifest(
    query: &str,
    search_bin: bool,
    manifest_path: &Path,
) -> Result<Option<SearchMatch>> {
    let name = leaf_base(manifest_path);
    // substring check on app_name
    if name.contains(query) {
        match Manifest::from_path(manifest_path) {
            Ok(manifest) => {
                let version = manifest.data.version;
                Ok(Some(SearchMatch {
                    name,
                    version,
                    bin: None,
                }))
            }
            Err(e) => Err(e),
        }
    } else {
        // Searching binaries requires a very-high overhead (reading all json files),
        // will not do binary search without the option.
        if !search_bin {
            return Ok(None);
        }

        match Manifest::from_path(manifest_path) {
            Ok(manifest) => {
                let Manifest {
                    name,
                    path: _,
                    bucket: _,
                    data,
                } = manifest;

                let bin_matches = try_match_bin(query, data.bin);
                if bin_matches.len() > 0 {
                    let version = data.version;
                    let bin = format!("'{}'", bin_matches[0].to_string());
                    Ok(Some(SearchMatch {
                        name,
                        version,
                        bin: Some(bin),
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(e),
        }
    }
}

impl<'a> Scoop<'a> {
    pub fn search(&mut self, query: &str, search_bin: bool) -> Result<()> {
        // Load all local buckets
        let buckets = self.bucket_manager.get_buckets();

        let mut matches: Vec<Matches> = Vec::new();

        buckets.iter().for_each(|(bucket_name, bucket)| {
            let manifests = bucket.available_manifests().unwrap();
            let search_matches = Arc::new(Mutex::new(Vec::new()));

            manifests.par_iter().for_each(|manifest_path| {
                match travel_manifest(query, search_bin, manifest_path).unwrap() {
                    Some(sm) => search_matches.lock().unwrap().push(sm),
                    None => {}
                }
            });

            let mut collected = search_matches.lock().unwrap().to_vec();
            collected.sort_by_key(|s| s.name.to_string());

            matches.push(Matches {
                bucket: bucket_name.to_string(),
                collected,
            });
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
