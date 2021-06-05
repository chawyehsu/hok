use scoop_core::{manifest, Scoop};
use clap::ArgMatches;
use serde_json::Value;

pub fn cmd_info(matches: &ArgMatches, scoop: &mut Scoop) {
    let app = matches.value_of("app").unwrap();
    match scoop.find_local_manifest(app) {
        Ok(Some(manifest)) => {
            // Name
            println!("Name: {}", manifest.app);
            // Bucket
            if manifest.bucket.is_some() {
                println!("Bucket: {}", manifest.bucket.unwrap());
            }
            // Description
            if manifest.json.get("description").is_some() {
                println!(
                    "Description: {}",
                    manifest.json.get("description").unwrap().as_str().unwrap()
                );
            }
            // Version
            println!("Version: {}", manifest.version);
            // Homepage
            if manifest.json.get("homepage").is_some() {
                println!(
                    "Website: {}",
                    manifest.json.get("homepage").unwrap().as_str().unwrap()
                );
            }
            // License
            if manifest.license.is_some() {
                let licenses = manifest.license.unwrap();

                if licenses.len() == 1 {
                    print!("License:");
                    let pair = licenses.first().unwrap();
                    match pair.1.as_ref() {
                        Some(url) => print!(" {} ({})\n", pair.0, url),
                        None => print!(" {}\n", pair.0),
                    }
                } else {
                    println!("License:");
                    for pair in licenses {
                        match pair.1 {
                            Some(url) => println!("  {} ({})", pair.0, url),
                            None => println!("  {}", pair.0),
                        }
                    }
                }
            }
            // Manifest
            match manifest.kind {
                manifest::ManifestKind::Local(path) => {
                    println!("Manifest: \n  {}", path.to_str().unwrap());
                }
                manifest::ManifestKind::Remote(_url) => {} // FIXME
            }
            // Binaries
            match manifest.json.get("bin") {
                Some(Value::String(single)) => {
                    println!("Binaries: \n  {}", single);
                }
                Some(Value::Array(_multiple)) => {
                    println!("Binaries:");
                    // for s in multiple {
                    //   match s {
                    //     Value::String(s) =>
                    //   }
                    // }
                }
                _ => {} // no-op
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
