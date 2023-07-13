use clap::ArgMatches;
use scoop_core::Session;

use crate::Result;

pub fn cmd_unhold(matches: &ArgMatches, session: &Session) -> Result<()> {
    let query = matches
        .values_of("package")
        .map(|v| v.collect::<Vec<_>>())
        .unwrap_or_default()
        .join(" ");

    // if let Some(name) = matches.value_of("app") {
    //     let app_manager = AppManager::new(config);
    //     if app_manager.is_app_installed(name) {
    //         match app_manager.get_app(name).unhold() {
    //             Ok(..) => println!("{} is no longer held and can be updated again.", name),
    //             Err(..) => eprintln!("failed to unhold {}", name),
    //         }
    //     }
    // }
    Ok(())
}
