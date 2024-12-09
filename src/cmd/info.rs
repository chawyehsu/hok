use clap::Parser;
use libscoop::{operation, Session};

use crate::Result;

/// Show package(s) basic information
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The query string (regex supported)
    query: String,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let query = args.query;

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
    Ok(())
}
