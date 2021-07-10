use std::sync::{Arc, Mutex};

use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use crate::Config;
use crate::ScoopResult;
use crate::{manager::BucketManager, model::App};

/// Main method for `scoop search`.
pub fn search<'cfg>(
    config: &'cfg Config,
    query: &str,
) -> ScoopResult<Vec<(String, Vec<(App<'cfg>, Option<String>)>)>> {
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
                let bin = app.search_bin(query);
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
