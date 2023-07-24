#![allow(unused)]
use clap::ArgMatches;
use libscoop::{operation, Session, SyncOption};
use std::collections::HashSet;

use crate::Result;

pub fn cmd_uninstall(matches: &ArgMatches, session: &Session) -> Result<()> {
    let queries = matches
        .get_many::<String>("package")
        .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut options = vec![SyncOption::Remove];
    operation::package_sync(session, queries, options)?;
    eprintln!("Not implemented yet.");
    Ok(())
}
