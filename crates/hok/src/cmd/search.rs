use clap::ArgMatches;
use crossterm::style::Stylize;
use libscoop::{operation, QueryOption, Session};

use crate::Result;

pub fn cmd_search(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(queries) = matches.get_many::<String>("query") {
        let queries = queries.map(|s| s.as_str()).collect::<Vec<_>>();
        let mut options = vec![];

        if matches.get_flag("with-binary") {
            options.push(QueryOption::Binary);
        }

        if matches.get_flag("with-description") {
            options.push(QueryOption::Description);
        }

        // Sort the results by name.
        let mut packages = operation::package_search(session, queries, options.clone())?;
        packages.sort_by_key(|pkg| pkg.name.clone());

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

            if pkg.installed() {
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

            if options.contains(&QueryOption::Description) {
                let description = pkg.description().unwrap_or("<no description>");
                output.push_str(format!("\n  {}", description).as_str());
            }

            if options.contains(&QueryOption::Binary) {
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
