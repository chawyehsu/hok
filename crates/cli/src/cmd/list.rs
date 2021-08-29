use crate::error::CliResult;
use clap::ArgMatches;
use scoop_core::ops::app::installed_apps;
use scoop_core::Config;

pub fn cmd_list(_matches: &ArgMatches, config: &Config) -> CliResult<()> {
    let apps = installed_apps(config)?;
    if apps.len() > 0 {
        println!("Installed apps:");
        for app in apps {
            // name and version
            print!("{} {}", app.name(), app.version());
            // bucket
            print!(" [{}]", app.bucket());
            print!("\n");
        }
    }
    Ok(())
}
