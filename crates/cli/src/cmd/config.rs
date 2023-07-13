use clap::ArgMatches;
use scoop_core::Session;
use std::process::Command;

use crate::Result;

pub fn cmd_config(matches: &ArgMatches, session: &mut Session) -> Result<()> {
    match matches.subcommand() {
        ("edit", Some(_)) => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "notepad".to_string());
            let mut child = Command::new(editor.as_str())
                .arg(&session.config.config_path)
                .spawn()?;
            child.wait()?;
            Ok(())
        }
        ("list", Some(_)) => {
            let config_json = session.config_list()?;
            println!("{}", config_json);
            Ok(())
        }
        ("set", Some(args)) => {
            let key = args.value_of("key").unwrap_or_default();
            let value = args.value_of("value").unwrap_or_default();
            session.config_set(key, value)?;
            Ok(())
        }
        ("unset", Some(args)) => {
            let key = args.value_of("key").unwrap_or_default();
            session.config_unset(key)?;
            Ok(())
        }
        _ => unreachable!(),
    }
}
