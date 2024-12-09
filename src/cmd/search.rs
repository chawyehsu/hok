use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use libscoop::{operation, QueryOption, Session};

use crate::Result;

/// Search available package(s)
///
/// Search available package(s) from synced buckets.
/// The query is performed against package names by default, use
/// --with-description or --with-binary to search through package
/// descriptions or binaries.
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The query string (regex supported by default)
    #[arg(required = true, action = ArgAction::Append)]
    query: Vec<String>,
    /// Turn regex off and use explicit matching
    #[arg(short = 'e', long, action = ArgAction::SetTrue, conflicts_with_all = &["with_binary", "with_description"])]
    explicit: bool,
    /// Search through package binaries as well
    #[arg(short = 'B', long, action = ArgAction::SetTrue)]
    with_binary: bool,
    /// Search through package descriptions as well
    #[arg(short = 'D', long, action = ArgAction::SetTrue)]
    with_description: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let queries = args.query.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let mut options = vec![];

    if args.with_binary {
        options.push(QueryOption::Binary);
    }

    if args.with_description {
        options.push(QueryOption::Description);
    }

    if args.explicit {
        options.push(QueryOption::Explicit);
    }

    let packages = operation::package_query(session, queries, options, false)?;

    for pkg in packages {
        let mut output = String::new();
        output.push_str(
            format!("{}/{} {}", pkg.name(), pkg.bucket().green(), pkg.version()).as_str(),
        );

        if pkg.is_strictly_installed() {
            let manifest_version = pkg.version();
            let installed_version = pkg.installed_version().unwrap();
            if manifest_version != installed_version {
                output.push_str(
                    format!(" [installed: {}]", installed_version)
                        .blue()
                        .to_string()
                        .as_str(),
                );
            } else {
                output.push_str(" [installed]".blue().to_string().as_str());
            }
        }

        if args.with_description {
            let description = pkg.description().unwrap_or("<no description>");
            output.push_str(format!("\n  {}", description).as_str());
        }

        if args.with_binary {
            let shims = match pkg.shims() {
                None => "<no shims>".to_owned(),
                Some(shims) => shims.join(","),
            };
            output.push_str(format!("\n  {}", shims).as_str());
        }

        println!("{}", output);
    }
    Ok(())
}
