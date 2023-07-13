use clap::ArgMatches;
use console::{style, Term};
use scoop_core::Session;

use crate::Result;

pub fn cmd_list(matches: &ArgMatches, session: &Session) -> Result<()> {
    let query = matches
        .values_of("package")
        .map(|v| v.collect::<Vec<_>>())
        .unwrap_or_default()
        .join(" ");
    let flag_upgradable = matches.is_present("upgradable");
    match session.package_list(&query, flag_upgradable) {
        Err(e) => Err(e.into()),
        Ok(packages) => {
            let term = Term::stdout();
            for pkg in packages {
                let name = pkg.name.as_str();
                let bucket = pkg.bucket.as_str();
                let version = pkg.version.as_str();

                let held = pkg.is_held();
                let upgradable = pkg.upgradable();

                if held {
                    let _ = match upgradable {
                        None => term.write_line(&format!(
                            "{}/{} {} [{}]",
                            name,
                            style(bucket).green(),
                            version,
                            style("held").magenta()
                        )),
                        Some(upgradable_version) => term.write_line(&format!(
                            "{}/{} {} [{},upgradable to {}]",
                            name,
                            style(bucket).green(),
                            version,
                            style("held").magenta(),
                            upgradable_version
                        )),
                    };
                } else {
                    let _ = match upgradable {
                        None => term.write_line(&format!(
                            "{}/{} {}",
                            name,
                            style(bucket).green(),
                            version
                        )),
                        Some(upgradable_version) => term.write_line(&format!(
                            "{}/{} {} [upgradable to {}]",
                            name,
                            style(bucket).green(),
                            version,
                            upgradable_version
                        )),
                    };
                }
            }
            Ok(())
        }
    }
}
