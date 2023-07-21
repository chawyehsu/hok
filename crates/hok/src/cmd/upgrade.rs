use clap::ArgMatches;
use libscoop::Session;

use crate::Result;

pub fn cmd_upgrade(matches: &ArgMatches, session: &Session) -> Result<()> {
    let _query = matches
        .get_many::<String>("package")
        .map(|v| v.map(|s| s.to_owned()).collect::<Vec<_>>())
        .unwrap_or_default()
        .join(" ");
    let _ = session;
    eprintln!("Not implemented yet.");
    Ok(())
}
