#![allow(unused)]
use clap::ArgMatches;
use libscoop::Session;
use std::collections::HashSet;

use crate::Result;

pub fn cmd_install(matches: &ArgMatches, session: &Session) -> Result<()> {
    let queries = matches
        .get_many::<String>("package")
        .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    // let options = HashSet::new();
    // operation::package_install(session, queries, options)?;
    eprintln!("Not implemented yet.");
    Ok(())
}
