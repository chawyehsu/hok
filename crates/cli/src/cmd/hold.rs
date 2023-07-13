use clap::ArgMatches;
use scoop_core::Session;

use crate::Result;

pub fn cmd_hold(matches: &ArgMatches, session: &Session) -> Result<()> {
    let query = matches
        .values_of("package")
        .map(|v| v.collect::<Vec<_>>())
        .unwrap_or_default()
        .join(" ");

    // if let Some(name) = matches.value_of("package") {
    //     let app_manager = AppManager::new(config);
    //     if app_manager.is_app_installed(name) {
    //         match app_manager.get_app(name).hold() {
    //             Ok(..) => println!("{} is now held and can not be updated anymore.", name),
    //             Err(..) => eprintln!("failed to hold {}", name),
    //         }
    //     }
    // }
    Ok(())
}
