use scoop_core::{AppManager, Config};

pub fn cmd_unhold(matches: &clap::ArgMatches, config: &Config) {
    if let Some(name) = matches.value_of("app") {
        let app_manager = AppManager::new(config);
        if app_manager.is_app_installed(name) {
            match app_manager.get_app(name).unhold() {
                Ok(..) => println!("{} is no longer held and can be updated again.", name),
                Err(..) => eprintln!("failed to unhold {}", name),
            }
        }
    }
}
