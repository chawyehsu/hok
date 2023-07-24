use clap::ArgMatches;
use crossterm::style::Stylize;
use libscoop::{operation, QueryOption, Session};

use crate::Result;

pub fn cmd_search(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(queries) = matches.get_many::<String>("query") {
        let queries = queries.map(|s| s.as_str()).collect::<Vec<_>>();
        let mut options = vec![];
        let with_binary = matches.get_flag("with-binary");
        let with_description = matches.get_flag("with-description");

        if with_binary {
            options.push(QueryOption::Binary);
        }

        if with_description {
            options.push(QueryOption::Description);
        }

        if matches.get_flag("explicit") {
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

            if with_description {
                let description = pkg.description().unwrap_or("<no description>");
                output.push_str(format!("\n  {}", description).as_str());
            }

            if with_binary {
                let shims = match pkg.shims() {
                    None => "<no shims>".to_owned(),
                    Some(shims) => shims.join(","),
                };
                output.push_str(format!("\n  {}", shims).as_str());
            }

            println!("{}", output);
        }
    }
    Ok(())
}
