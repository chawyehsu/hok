use std::path::{Path, PathBuf};

use crate::{Result, fs::{leaf, leaf_base}, manifest::{Bins, Manifest}};

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

fn try_match_bin(query: &str, input: Option<Bins>) -> Option<String> {
    if input.is_some() {
        let bins = input.unwrap();
        for bin in bins.iter() {
            let bin_name = leaf(PathBuf::from(bin[0].as_str()).as_path());
            if bin_name.contains(query) {
                return Some(bin_name);
            }
        }
    }

    None
}

pub(crate) fn travel_manifest(
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
                let name = manifest.name;
                let data = manifest.data;

                let bin_match = try_match_bin(query, data.bin);
                if bin_match.is_some() {
                    let name = name;
                    let version = data.version;
                    let bin = format!("'{}'", bin_match.unwrap());
                    return Ok(Some(SearchMatch {
                        name,
                        version,
                        bin: Some(bin),
                    }));
                }

                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}
