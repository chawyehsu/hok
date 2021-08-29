use crate::manager::BucketManager;
use crate::model::AvailableApp;
use crate::model::InstallInfo;
use crate::model::InstalledApp;
use crate::Config;
use crate::ScoopResult;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;

pub fn installed_apps<'cfg>(config: &'cfg Config) -> ScoopResult<Vec<InstalledApp<'cfg>>> {
    let mut apps = config
        .apps_path()
        .read_dir()?
        .par_bridge()
        .filter_map(|de| {
            if de.is_err() {
                return None;
            }
            let de = de.unwrap();
            if !de.file_type().unwrap().is_dir() {
                log::debug!(
                    "surprising entry of {} is in apps folder",
                    de.path().display()
                );
                return None;
            }
            let file_name = de.file_name();
            let name = file_name.to_str().unwrap();

            let mut keg = config.apps_path().join(name);
            keg.push("current");
            keg.push("install.json");
            log::trace!("{}", keg.display());
            if !keg.exists() {
                log::debug!("broken installation of app {}", name);
                return None;
            }

            match InstallInfo::new(keg) {
                Ok(info) => {
                    // filter orphan apps that do not have bucket specified
                    if info.bucket().is_some() {
                        let bucket = info.bucket().unwrap();
                        return Some(InstalledApp::new(config, name, bucket).unwrap());
                    } else {
                        return None;
                    }
                }
                Err(_) => None,
            }
        })
        .collect::<Vec<_>>();
    if apps.len() > 0 {
        apps.sort_by_key(|app| app.name().to_owned());
    }
    Ok(apps)
}

/// Search available app
pub fn search_available_app<'cfg, S>(
    config: &'cfg Config,
    pattern: S,
) -> ScoopResult<AvailableApp<'cfg>>
where
    S: AsRef<str>,
{
    let pattern = pattern.as_ref();
    let bucket_manager = BucketManager::new(config);
    // Cehck the given pattern whether having bucket name prefix
    let (bucket_name, app_name) = match pattern.contains("/") {
        true => pattern.split_once("/").map(|(a, b)| (Some(a), b)).unwrap(),
        false => (None, pattern),
    };

    match bucket_name {
        Some(bucket_name) => match bucket_manager.bucket(bucket_name) {
            Some(bucket) => {
                return bucket.app(app_name);
            }
            None => anyhow::bail!("could no find bucket named '{}'", bucket_name),
        },
        None => {
            for bucket in bucket_manager.buckets().into_iter() {
                if bucket.contains_app(app_name) {
                    return bucket.app(app_name);
                }
            }
        }
    }
    anyhow::bail!("no available app with the name '{}'", pattern);
}
