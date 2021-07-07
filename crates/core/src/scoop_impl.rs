use crate::{
    error::ScoopResult,
    manifest::Manifest,
    search::{travel_manifest, Matches},
    BucketManager, Config,
};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

/// Find and return local manifest represented as [`Manifest`], using given
/// `pattern`.
pub fn find_manifest<S>(config: &Config, pattern: S) -> ScoopResult<Option<Manifest>>
where
    S: AsRef<str>,
{
    let bucket_manager = BucketManager::new(config);

    // Detect given pattern whether having bucket name prefix
    let (bucket_name, app_name) = match pattern.as_ref().contains("/") {
        true => {
            let (a, b) = pattern.as_ref().split_once("/").unwrap();
            (Some(a), b)
        }
        false => (None, pattern.as_ref()),
    };

    match bucket_name {
        Some(bucket_name) => {
            let bucket = bucket_manager.bucket(bucket_name).unwrap();
            let manifest_path = bucket.manifest_root().join(format!("{}.json", app_name));
            match manifest_path.exists() {
                true => Ok(Some(Manifest::from_path(&manifest_path)?)),
                false => Ok(None),
            }
        }
        None => {
            for (_, bucket) in bucket_manager.buckets().iter() {
                let manifest_path = bucket.manifest_root().join(format!("{}.json", app_name));
                if manifest_path.exists() {
                    return Ok(Some(Manifest::from_path(&manifest_path)?));
                }
            }

            Ok(None)
        }
    }
}

pub fn search(config: &Config, query: &str, search_bin: bool) -> ScoopResult<Vec<Matches>> {
    let bucket_manager = BucketManager::new(config);
    // Load all local buckets
    let buckets = bucket_manager.buckets();

    let mut matches: Vec<Matches> = Vec::new();

    buckets.iter().for_each(|(bucket_name, bucket)| {
        let manifests = bucket.manifests().unwrap();
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
