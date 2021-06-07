pub fn cmd_unhold(matches: &clap::ArgMatches, scoop: &mut scoop_core::Scoop) {
    if let Some(name) = matches.value_of("app") {
        if scoop.app_manager.is_app_installed(name) {
            match scoop.app_manager.get_app(name).unhold() {
                Ok(..) => println!("{} is no longer held and can be updated again.", name),
                Err(..) => eprintln!("failed to unhold {}", name),
            }
        }
    }
}
