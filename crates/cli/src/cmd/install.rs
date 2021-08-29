use crate::console;
use crate::error::CliResult;
use clap::ArgMatches;
use scoop_core::ops::app::search_available_app;
use scoop_core::ops::install::{install, resolve_install_order};
use scoop_core::util::block_on;
use scoop_core::{manager::AppManager, Config};

pub fn cmd_install(matches: &ArgMatches, config: &Config) -> CliResult<()> {
    if let Some(arg_apps) = matches.values_of("app") {
        let ignore_cache = matches.is_present("ignore_cache");
        let skip_hash_validation = matches.is_present("skip_hash_validation");

        let app_manager = AppManager::new(config);
        let apps = arg_apps.into_iter().collect::<Vec<_>>();
        let ordered_apps = resolve_install_order(config, apps.clone())?;
        let mut apps_to_install = vec![];

        for pattern in ordered_apps {
            let (_, app_name) = match pattern.contains("/") {
                true => pattern.split_once("/").unwrap(),
                false => ("", pattern.as_str()),
            };

            // check installed apps
            let is_installed = app_manager.is_app_installed(app_name);
            if is_installed && apps.contains(&pattern.as_str()) {
                let msg = format!(
                    "{} is already installed\n\
                    To upgrade {}, run:\n  scoop upgrade {}",
                    app_name, app_name, app_name
                );
                console::warn(msg.as_str())?;
            }

            if !is_installed {
                let app = search_available_app(config, pattern)?;
                apps_to_install.push(app);
            }
        }

        for app in apps_to_install {
            println!("Installing '{}' ({})", app.name(), app.manifest().version());
            block_on(install(config, &app, ignore_cache, skip_hash_validation))?;
        }
    }
    Ok(())
}
