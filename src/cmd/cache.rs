use clap::{ArgAction, Parser, Subcommand};
use libscoop::{operation, Session};

use crate::{util, Result};

/// Package cache management
#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List download caches
    #[clap(alias = "ls")]
    List {
        /// List caches matching the query
        query: Option<String>,
    },
    /// Remove download caches
    #[clap(alias = "rm")]
    #[clap(arg_required_else_help = true)]
    Remove {
        /// Remove caches matching the query
        query: Option<String>,
        /// Remove all caches
        #[arg(short, long, action = ArgAction::SetTrue, conflicts_with = "query")]
        all: bool,
    },
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    match args.command {
        Command::List { query } => {
            let query = query.unwrap_or("*".to_string());
            let files = operation::cache_list(session, query.as_str())?;
            let mut total_size: u64 = 0;
            let total_count = files.len();

            for f in files.into_iter() {
                let size = f.path().metadata()?.len();
                total_size += size;

                println!(
                    "{:>8} {} ({}) {:>}",
                    util::humansize(size, true),
                    f.package_name(),
                    f.version(),
                    f.file_name()
                );
            }

            println!(
                "{:>8} {} files, {}",
                "Total:",
                total_count,
                util::humansize(total_size, true)
            );

            Ok(())
        }
        Command::Remove { query, all } => {
            if all {
                match operation::cache_remove(session, "*") {
                    Ok(_) => {
                        println!("All download caches were removed.");
                        return Ok(());
                    }
                    Err(e) => return Err(e.into()),
                }
            }

            if let Some(query) = query {
                match operation::cache_remove(session, query.as_str()) {
                    Ok(_) => {
                        if query == "*" {
                            println!("All download caches were removed.");
                        } else {
                            println!("All caches matching '{}' were removed.", query);
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            }

            Ok(())
        }
    }
}
