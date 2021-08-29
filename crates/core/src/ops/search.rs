use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use crate::manager::BucketManager;
use crate::model::AvailableApp;
use crate::util::leaf;
use crate::Config;
use crate::ScoopResult;

/// Main method for `scoop search`.
pub fn search<'cfg>(
    config: &'cfg Config,
    query: &str,
) -> ScoopResult<Vec<(String, Vec<(AvailableApp<'cfg>, Option<String>)>)>> {
    let bucket_manager = BucketManager::new(config);
    let res = Arc::new(Mutex::new(Vec::new()));
    bucket_manager.buckets().into_par_iter().for_each(|bucket| {
        let apps = Arc::new(Mutex::new(Vec::new()));
        bucket.apps().unwrap().into_par_iter().for_each(|app| {
            if app.name().contains(query) {
                // 1. search app name
                apps.lock().unwrap().push((app, None));
            } else {
                // 2. search app bin
                let bin = search_bin(query, &app);
                if bin.is_some() {
                    apps.lock().unwrap().push((app, bin));
                }
            }
        });
        if apps.lock().unwrap().len() > 0 {
            let bucket_name = bucket.name().to_owned();
            let mut apps = Arc::try_unwrap(apps).unwrap().into_inner().unwrap();
            // sort by app name
            apps.sort_by_key(|app| app.0.name().to_owned());
            res.lock().unwrap().push((bucket_name, apps));
        }
    });

    if res.lock().unwrap().len() > 0 {
        res.lock()
            .unwrap()
            .sort_by_key(|(bucket_name, _)| bucket_name.to_owned());
    }

    Ok(Arc::try_unwrap(res).unwrap().into_inner().unwrap())
}

/// Search App's bin.
fn search_bin(query: &str, app: &AvailableApp) -> Option<String> {
    match app.manifest().bin() {
        Some(bins) => {
            for bin in bins.iter() {
                let length = bin.len();
                if length > 0 {
                    // the first is the original name
                    let leaf_bin = leaf(&PathBuf::from(bin[0].clone()));
                    if leaf_bin.contains(query) {
                        return Some(leaf_bin);
                    }
                }
                if length > 1 {
                    // the second is the shim name
                    if bin[1].contains(query) {
                        return Some(bin[1].clone());
                    }
                }
            }
        }
        None => {}
    }
    None
}
