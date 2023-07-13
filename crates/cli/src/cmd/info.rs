use clap::ArgMatches;
use scoop_core::Session;

use crate::Result;

pub fn cmd_info(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(query) = matches.value_of("package") {
        let options = "explicit";
        let packages = session.package_search(query, options)?;
        let length = packages.len();
        match length {
            0 => eprintln!("Could not find package named '{}'.", query),
            _ => {
                println!("Found {} package(s) named '{}':", length, query);
                for (idx, pkg) in packages.iter().enumerate() {
                    // Ident
                    // println!("Identity: {}/{}", pkg.bucket, pkg.name);
                    // Name
                    println!("Name: {}", pkg.name);
                    // Bucket
                    println!("Bucket: {}", pkg.bucket);
                    // Description
                    println!(
                        "Description: {}",
                        pkg.description().unwrap_or("<no description>".to_owned())
                    );
                    // Version
                    println!("Version: {}", pkg.version);
                    // Homepage
                    println!("Homepage: {}", pkg.homepage());
                    // License
                    // println!("License: {}", pkg.license);
                    // Binaries
                    println!(
                        "Shims: {}",
                        pkg.shims()
                            .map(|v| v.join(","))
                            .unwrap_or("<no shims>".to_owned())
                    );

                    if idx != (length - 1) {
                        println!("");
                    }
                }
            }
        }
    }
    Ok(())
}
