use clap::{Parser, Subcommand};
use crossterm::style::Stylize;
use libscoop::{operation, Session};

use crate::Result;

/// Configuration management
#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Edit the config file [default editor: notepad]
    Edit,
    /// List all settings in key-value
    #[clap(alias = "ls")]
    List,
    /// Add a new setting to the config file
    #[clap(arg_required_else_help = true)]
    Set {
        /// The key of the config
        key: String,
        /// The value of the setting
        value: String,
    },
    /// Remove a setting from config file
    #[clap(arg_required_else_help = true)]
    Unset {
        /// The key of the setting
        key: String,
    },
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    match args.command {
        Command::Edit => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "notepad".to_string());
            let mut child = std::process::Command::new(editor.as_str())
                .arg(&session.config().path)
                .spawn()?;
            child.wait()?;
            Ok(())
        }
        Command::List => {
            let config_json = operation::config_list(session)?;
            println!("{}:", &session.config().path.display().to_string().green());
            println!("{}", config_json);
            Ok(())
        }
        Command::Set { key, value } => {
            operation::config_set(session, key.as_str(), value.as_str())?;
            println!("Config '{}' has been set to '{}'", key, value);
            Ok(())
        }
        Command::Unset { key } => {
            operation::config_set(session, key.as_str(), "")?;
            println!("Config '{}' has been unset", key);
            Ok(())
        }
    }
}
