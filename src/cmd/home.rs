use clap::Parser;
use libscoop::{operation, QueryOption, Session};
use std::{io::Write, process::Command};

use crate::Result;

/// Browse the homepage of a package
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package name
    package: String,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let query = args.package;

    let queries = vec![query.as_str()];
    let options = vec![QueryOption::Explicit];
    let mut result = operation::package_query(session, queries, options, false)?;

    match result.len() {
        0 => eprintln!("Could not find package named '{}'.", query),
        1 => {
            let package = &result[0];
            let url = std::ffi::OsStr::new(package.homepage());
            Command::new("cmd")
                .arg("/C")
                .arg("start")
                .arg(url)
                .spawn()?;
        }
        _ => {
            result.sort_by_key(|p| p.ident());

            println!("Found multiple packages named '{}':\n", query);
            for (idx, pkg) in result.iter().enumerate() {
                println!(
                    "  {}. {}/{} ({})",
                    idx,
                    pkg.bucket(),
                    pkg.name(),
                    pkg.homepage()
                );
            }
            print!("\nPlease select one, enter the number to continue: ");
            std::io::stdout().flush().unwrap();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            let parsed = input.trim().parse::<usize>();
            if let Ok(num) = parsed {
                if num < result.len() {
                    let package = &result[num];
                    let url = std::ffi::OsStr::new(package.homepage());
                    Command::new("cmd")
                        .arg("/C")
                        .arg("start")
                        .arg(url)
                        .spawn()?;
                    return Ok(());
                }
            }
            eprintln!("Invalid input.");
        }
    }
    Ok(())
}
