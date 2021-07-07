use std::path::PathBuf;

use clap::ArgMatches;
use scoop_core::{find_manifest, fs::leaf, Config};

pub fn cmd_info(matches: &ArgMatches, config: &Config) {
    let app = matches.value_of("app").unwrap();
    match find_manifest(&config, app) {
        Ok(Some(manifest)) => {
            // Name
            println!("Name: {}", manifest.get_name());
            // Bucket
            println!("Bucket: {}", manifest.get_bucket());
            // Description
            if let Some(description) = manifest.get_description() {
                println!("Description: {}", description);
            }
            // Version
            println!("Version: {}", manifest.get_version());
            // Homepage
            if let Some(homepage) = manifest.get_homepage() {
                println!("Website: {}", homepage);
            }
            // License
            if let Some(license) = manifest.get_license() {
                let identifier = license.identifier();

                if license.url().is_some() {
                    let url = license.url().unwrap();
                    println!("License:\n  {} ({})", identifier, url);
                } else {
                    println!("License: {}", identifier);
                }
            }
            // Manifest
            println!("Manifest:\n  {}", manifest.path().display());

            // FIXME: check data.architecture.<arch>.bin
            // Binaries
            if let Some(bins) = manifest.get_bin() {
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

            std::process::exit(0);
        }
        Ok(None) => {
            eprintln!("Could not find manifest for '{}'", app);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to operate. ({})", e);
            std::process::exit(1);
        }
    }
}
