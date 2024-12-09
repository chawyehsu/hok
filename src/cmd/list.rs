use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use libscoop::{operation, QueryOption, Session};

use crate::Result;

/// List installed package(s)
#[derive(Debug, Parser)]
pub struct Args {
    /// The query string (regex supported by default)
    #[arg(action = ArgAction::Append)]
    query: Vec<String>,
    /// Turn regex off and use explicit matching
    #[arg(short = 'e', long, action = ArgAction::SetTrue)]
    explicit: bool,
    /// List upgradable package(s)
    #[arg(short = 'u', long, action = ArgAction::SetTrue)]
    upgradable: bool,
    /// List held package(s)
    #[arg(short = 'H', long, action = ArgAction::SetTrue)]
    held: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let queries = args.query.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let mut options = vec![];

    if args.explicit {
        options.push(QueryOption::Explicit);
    }

    if args.upgradable {
        options.push(QueryOption::Upgradable);
    }

    match operation::package_query(session, queries, options, true) {
        Err(e) => Err(e.into()),
        Ok(packages) => {
            for pkg in packages {
                let mut output = String::new();
                output.push_str(
                    format!("{}/{} {}", pkg.name(), pkg.bucket().green(), pkg.version()).as_str(),
                );

                let held = pkg.is_held();
                if args.held && !held {
                    continue;
                }

                let upgradable = pkg.upgradable_version();
                if args.upgradable && upgradable.is_some() {
                    output.push_str(format!(" -> {}", upgradable.unwrap().blue()).as_str());
                }

                if held {
                    output.push_str(format!(" [{}]", "held".magenta()).as_str());
                }

                println!("{}", output);
            }
            Ok(())
        }
    }
}
