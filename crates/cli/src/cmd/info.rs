use std::path::PathBuf;

use clap::ArgMatches;
use scoop_core::{License, Scoop, fs::leaf};

pub fn cmd_info(matches: &ArgMatches, scoop: &mut Scoop) {
    let app = matches.value_of("app").unwrap();
    match scoop.find_local_manifest(app) {
        Ok(Some(manifest)) => {
            let name = manifest.name;
            let path = manifest.path;
            let bucket = manifest.bucket;
            let data = manifest.data;

            // Name
            println!("Name: {}", name);
            // Bucket
            if bucket.is_some() {
                println!("Bucket: {}", bucket.unwrap());
            }
            // Description
            if data.description.is_some() {
                println!("Description: {}", data.description.unwrap());
            }
            // Version
            println!("Version: {}", data.version);
            // Homepage
            if data.homepage.is_some() {
                println!("Website: {}", data.homepage.unwrap());
            }
            // License
            if data.license.is_some() {
                let licenses = data.license.unwrap();

                match licenses {
                    License::Simple(str) => println!("License: {}", str),
                    License::Complex(pair) => {
                        println!("License:");
                        match pair.url {
                            Some(url) => println!("  {} ({})", pair.identifier, url),
                            None => println!("  {}", pair.identifier),
                        }
                    }
                }
            }
            // Manifest
            println!("Manifest:\n  {}", path.display());

            // FIXME: check data.architecture.<arch>.bin
            // Binaries
            if data.bin.is_some() {
                let bins = data.bin.unwrap();
                if bins.len() == 1 {
                    let bin = bins[0][0].as_str();
                    println!("Binary: {}", bin);
                } else {
                    println!("Binaries:");
                    let out = bins.iter().map(|b| {
                        leaf(PathBuf::from(b[0].as_str()).as_path())
                    }).collect::<Vec<String>>();
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
