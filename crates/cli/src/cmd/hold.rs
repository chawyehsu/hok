use scoop_core::{manager::AppManager, Config};

pub fn cmd_hold(matches: &clap::ArgMatches, config: &Config) {
    if let Some(name) = matches.value_of("app") {
        let app_manager = AppManager::new(config);
        if app_manager.is_app_installed(name) {
            match app_manager.get_app(name).hold() {
                Ok(..) => println!("{} is now held and can not be updated anymore.", name),
                Err(..) => eprintln!("failed to hold {}", name),
            }
        }
    }
}
