use clap::ArgMatches;
use scoop_core::Session;

use crate::Result;

pub fn cmd_search(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(queries) = matches.values_of("query") {
        let query = queries.collect::<Vec<&str>>().join(" ");
        let mut options = "";
        if matches.is_present("explicit") {
            options = "explicit";
        }
        if matches.is_present("names-only") {
            options = "names-only";
        }
        if matches.is_present("with-binaries") {
            options = "with-binaries";
        }

        // Sort the results by name.
        let mut packages = session.package_search(&query, options)?;
        packages.sort_by_key(|pkg| pkg.name.clone());

        for pkg in packages {
            match options {
                "explicit" | "names-only" => {
                    println!("{}/{} {}", pkg.name, pkg.bucket, pkg.version);
                }
                "with-binaries" => {
                    let description = pkg.description().unwrap_or("<no description>".to_owned());
                    let shims = match pkg.shims() {
                        None => "<no shims>".to_owned(),
                        Some(shims) => shims.join(","),
                    };
                    println!(
                        "{}/{} {}\n  {}\n  {}",
                        pkg.name, pkg.bucket, pkg.version, description, shims
                    );
                }
                _ => {
                    let description = pkg.description().unwrap_or("<no description>".to_owned());
                    println!(
                        "{}/{} {}\n  {}",
                        pkg.name, pkg.bucket, pkg.version, description
                    );
                }
            }
        }
    }
    Ok(())
}
