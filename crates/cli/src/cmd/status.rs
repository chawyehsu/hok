use clap::ArgMatches;
use scoop_core::utils::compare_versions;
use std::cmp::Ordering;

pub fn cmd_status(_: &ArgMatches, scoop: &mut scoop_core::Scoop) {
    let installed_apps = scoop.app_manager.installed_apps();
    let mut outdated_apps = Vec::new();
    let mut onhold_apps = Vec::new();
    let mut removed_apps = Vec::new();
    let mut failed_apps = Vec::new();

    installed_apps.into_iter().for_each(|app| {
        let install_info = app.current_install_info().ok();
        match install_info {
            None => failed_apps.push(app.name()),
            Some(info) => {
                let cur_version = app.current_version();
                let is_hold = info.is_hold();

                if info.bucket.is_some() {
                    let pattern = format!("{}/{}", info.bucket.unwrap(), app.name());

                    match scoop.find_local_manifest(pattern).unwrap() {
                        None => removed_apps.push(app.name()),
                        Some(manifest) => {
                            let latest_version = manifest.get_version().to_owned();
                            match compare_versions(&latest_version, &cur_version) {
                                Ordering::Greater => {
                                    outdated_apps.push((
                                        app.name(),
                                        cur_version.clone(),
                                        latest_version.clone(),
                                    ));
                                    if is_hold {
                                        onhold_apps.push((app.name(), cur_version, latest_version));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    });

    if outdated_apps.len() > 0 {
        println!("Updates are available for:");
        outdated_apps.iter().for_each(|app| {
            println!("    {}: {} -> {}", app.0, app.1, app.2);
        })
    }

    if onhold_apps.len() > 0 {
        println!("These apps are outdated and on hold:");
        onhold_apps.iter().for_each(|app| {
            println!("    {}: {} -> {}", app.0, app.1, app.2);
        })
    }

    if removed_apps.len() > 0 {
        println!("These app manifests have been removed:");
        removed_apps.iter().for_each(|app| {
            println!("    {}", app);
        })
    }

    if failed_apps.len() > 0 {
        println!("These apps failed to install:");
        failed_apps.iter().for_each(|app| {
            println!("    {}", app);
        })
    }
}
