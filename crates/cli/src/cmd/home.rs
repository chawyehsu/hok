use clap::ArgMatches;
use scoop_core::{find_manifest, Config};
use std::process::Command;

pub fn cmd_home(matches: &ArgMatches, config: &Config) {
    if let Some(app_name) = matches.value_of("app") {
        // find local manifest and parse it
        match find_manifest(&config, app_name).unwrap() {
            Some(manifest) => {
                if let Some(url) = manifest.get_homepage() {
                    let url = std::ffi::OsStr::new(url.as_str());
                    Command::new("cmd")
                        .arg("/C")
                        .arg("start")
                        .arg(url)
                        .spawn()
                        .unwrap();
                } else {
                    println!("Could not find homepage in manifest for '{}'.", app_name);
                }
            }
            None => {
                println!("Could not find manifest for '{}'.", app_name);
            }
        }
    }
}
