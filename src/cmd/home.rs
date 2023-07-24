use clap::ArgMatches;
use libscoop::{operation, QueryOption, Session};
use std::process::Command;

use crate::Result;

pub fn cmd_home(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(query) = matches.get_one::<String>("package") {
        let queries = vec![query.as_str()];
        let options = vec![QueryOption::Explicit];
        let result = operation::package_query(session, queries, options, false)?;

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
                eprintln!("Found multiple packages named '{}':\n", query);
                for (idx, pkg) in result.iter().enumerate() {
                    println!(
                        "  {}. {}/{} ({})",
                        idx + 1,
                        pkg.bucket(),
                        pkg.name(),
                        pkg.homepage()
                    );
                }
                eprintln!("\nUse bucket prefix to narrow results.");
            }
        }
    }
    Ok(())
}
