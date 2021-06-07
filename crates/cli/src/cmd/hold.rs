pub fn cmd_hold(matches: &clap::ArgMatches, scoop: &mut scoop_core::Scoop) {
    if let Some(name) = matches.value_of("app") {
        if scoop.app_manager.is_app_installed(name) {
            match scoop.app_manager.get_app(name).hold() {
                Ok(..) => println!("{} is now held and can not be updated anymore.", name),
                Err(..) => eprintln!("failed to hold {}", name),
            }
        }
    }
}
