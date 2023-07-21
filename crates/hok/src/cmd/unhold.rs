use clap::ArgMatches;
use crossterm::style::Stylize;
use libscoop::{operation, Session};

use crate::Result;

pub fn cmd_unhold(matches: &ArgMatches, session: &Session) -> Result<()> {
    let packages = matches
        .get_many::<String>("package")
        .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    for name in packages {
        print!("Unholding {}... ", name);
        match operation::package_hold(session, name, false) {
            Ok(..) => {
                println!("{}", "Ok".green());
            }
            Err(err) => {
                println!("{}", "Err".red());
                return Err(err.into());
            }
        }
    }
    Ok(())
}
