use clap::ArgMatches;
use scoop_core::ops::app::search_available_app;
use scoop_core::util::leaf;
use scoop_core::Config;
use std::path::PathBuf;

use crate::error::CliResult;

pub fn cmd_info(matches: &ArgMatches, config: &Config) -> CliResult<()> {
    let arg_app = matches.value_of("app").unwrap();
    let app = search_available_app(&config, arg_app)?;

    // Name
    println!("Name: {}", app.name());
    // Bucket
    println!("Bucket: {}", app.bucket());
    let manifest = app.manifest();
    // Description
    if let Some(description) = manifest.description() {
        println!("Description: {}", description);
    }
    // Version
    println!("Version: {}", manifest.version());
    // Homepage
    if let Some(homepage) = manifest.homepage() {
        println!("Website: {}", homepage);
    }
    // License
    if let Some(license) = manifest.license() {
        let identifier = license.identifier();

        if license.url().is_some() {
            let url = license.url().unwrap();
            println!("License:\n  {} ({})", identifier, url);
        } else {
            println!("License: {}", identifier);
        }
    }
    // Manifest
    println!("Manifest:\n  {}", manifest.path());

    // FIXME: check data.architecture.<arch>.bin
    // Binaries
    if let Some(bins) = manifest.bin() {
        if bins.len() == 1 {
            let bin = bins[0][0].as_str();
            println!("Binary: {}", bin);
        } else {
            println!("Binaries:");
            let out = bins
                .iter()
                .map(|b| leaf(PathBuf::from(b[0].as_str()).as_path()))
                .collect::<Vec<String>>();
            println!("  {}", out.join(" "));
        }
    }
    Ok(())
}
