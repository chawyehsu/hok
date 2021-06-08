use std::path::{Path, PathBuf};

use crate::{
    fs::{leaf, leaf_base},
    manifest::{BinType, Manifest, StringOrStringArray},
    Result,
};

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
                let Manifest {
                    name,
                    path: _,
                    bucket: _,
                    data,
                } = manifest;

                let bin_match = try_match_bin(query, data.bin);
                if bin_match.is_some() {
                    let name = name;
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
