use clap::ArgMatches;
use libscoop::{operation, Session};

use crate::Result;

pub fn cmd_info(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(query) = matches.get_one::<String>("query") {
        let queries = vec![query.as_str()];
        let options = vec![];
        let packages = operation::package_query(session, queries, options, false)?;
        let length = packages.len();
        match length {
            0 => eprintln!("Could not find package for query '{}'.", query),
            _ => {
                if length == 1 {
                    println!("Found 1 package for query '{}':", query);
                } else {
                    println!("Found {} package(s) for query '{}':", length, query);
                }

                for (idx, pkg) in packages.iter().enumerate() {
                    // Ident
                    println!("Identity: {}", pkg.ident());
                    // Name
                    println!("Name: {}", pkg.name());
                    // Bucket
                    println!("Bucket: {}", pkg.bucket());
                    // Description
                    println!(
                        "Description: {}",
                        pkg.description().unwrap_or("<no description>")
                    );
                    // Version
                    println!("Version: {}", pkg.version());
                    // Homepage
                    println!("Homepage: {}", pkg.homepage());
                    // License
                    println!("License: {}", pkg.license());
                    // Binaries
                    println!(
                        "Shims: {}",
                        pkg.shims()
                            .map(|v| v.join(","))
                            .unwrap_or("<no shims>".to_owned())
                    );

                    if idx != (length - 1) {
                        println!();
                    }
                }
            }
        }
    }
    Ok(())
}
