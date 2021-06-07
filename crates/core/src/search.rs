use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{
    fs::{leaf, leaf_base},
    manifest::{BinType, Manifest, StringOrStringArray},
    Result, Scoop,
};
use rayon::prelude::*;

#[derive(Clone, Debug)]
pub struct SearchMatch {
    pub name: String,
    pub version: String,
    pub bin: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Matches {
    pub bucket: String,
    pub collected: Vec<SearchMatch>,
}

fn try_match_bin(query: &str, input: Option<BinType>) -> Option<String> {
    match input {
        None => {}
        Some(bintype) => match bintype {
            BinType::String(bin) => {
                let bin = leaf(PathBuf::from(bin).as_path());
                if bin.contains(query) {
                    return Some(bin);
                }
            }
            BinType::Array(arr) => {
                for item in arr.into_iter() {
                    match item {
                        StringOrStringArray::String(bin) => {
                            let bin = leaf(PathBuf::from(bin).as_path());
                            if bin.contains(query) {
                                return Some(bin);
                            }
                        }
                        StringOrStringArray::Array(pair) => {
                            let bin = leaf(PathBuf::from(pair[1].to_string()).as_path());
                            if bin.contains(query) {
                                return Some(bin);
                            }
                        }
                    }
                }
            }
        },
    }

    None
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

                let bin_match = try_match_bin(query, data.bin);
                if bin_match.is_some() {
                    let version = data.version;
                    let bin = format!("'{}'", bin_match.unwrap());
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
    pub fn search(&mut self, query: &str, search_bin: bool) -> Result<Vec<Matches>> {
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

        Ok(matches)
    }
}
