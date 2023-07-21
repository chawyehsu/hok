use clap::ArgMatches;
use crossterm::style::Stylize;
use libscoop::{operation, Session};

use crate::Result;

pub fn cmd_list(matches: &ArgMatches, session: &Session) -> Result<()> {
    let query = matches
        .get_many::<String>("package")
        .map(|v| v.map(|s| s.to_owned()).collect::<Vec<_>>())
        .unwrap_or_default()
        .join(" ");
    let flag_upgradable = matches.get_flag("upgradable");

    match operation::package_list(session, &query, flag_upgradable) {
        Err(e) => Err(e.into()),
        Ok(packages) => {
            for pkg in packages {
                let mut output = String::new();
                output.push_str(
                    format!(
                        "{}/{} {}",
                        pkg.name,
                        pkg.bucket.as_str().green(),
                        pkg.version()
                    )
                    .as_str(),
                );

                let held = pkg.is_held();
                let upgradable = pkg.upgradable();

                if upgradable.is_some() {
                    output.push_str(format!(" -> {}", upgradable.unwrap().blue()).as_str());
                    if held {
                        output.push_str(format!(" [{}]", "held".magenta()).as_str());
                    }
                }

                println!("{}", output);
            }
            Ok(())
        }
    }
}
