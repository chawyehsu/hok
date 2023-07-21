use clap::ArgMatches;
use libscoop::{operation, Session};
use std::process::Command;

use crate::Result;

pub fn cmd_config(matches: &ArgMatches, session: &Session) -> Result<()> {
    match matches.subcommand() {
        Some(("edit", _)) => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "notepad".to_string());
            let mut child = Command::new(editor.as_str())
                .arg(&session.get_config().config_path)
                .spawn()?;
            child.wait()?;
            Ok(())
        }
        Some(("list", _)) => {
            let config_json = operation::config_list(session)?;
            println!("{}", config_json);
            Ok(())
        }
        Some(("set", args)) => {
            let key = args
                .get_one::<String>("key")
                .map(|s| s.as_str())
                .unwrap_or_default();
            let value = args
                .get_one::<String>("value")
                .map(|s| s.as_str())
                .unwrap_or_default();
            operation::config_set(session, key, value)?;
            Ok(())
        }
        Some(("unset", args)) => {
            let key = args
                .get_one::<String>("key")
                .map(|s| s.as_str())
                .unwrap_or_default();
            operation::config_set(session, key, "")?;
            Ok(())
        }
        _ => unreachable!(),
    }
}
