use clap::ArgMatches;
use crossterm::style::Stylize;
use libscoop::{operation, QueryOption, Session};

use crate::Result;

pub fn cmd_list(matches: &ArgMatches, session: &Session) -> Result<()> {
    let queries = matches
        .get_many::<String>("query")
        .unwrap_or_default()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();
    let mut options = vec![];
    let flag_held = matches.get_flag("held");

    if matches.get_flag("explicit") {
        options.push(QueryOption::Explicit);
    }

    if matches.get_flag("upgradable") {
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
                if flag_held && !held {
                    continue;
                }

                let upgradable = pkg.upgradable();
                if upgradable.is_some() {
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
