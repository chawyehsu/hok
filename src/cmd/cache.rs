use clap::ArgMatches;
use libscoop::{operation, Session};

use crate::{util, Result};

pub fn cmd_cache(matches: &ArgMatches, session: &Session) -> Result<()> {
    match matches.subcommand() {
        Some(("list", args)) => {
            let query = args
                .get_one::<String>("query")
                .map(|s| s.as_ref())
                .unwrap_or("*");
            let files = operation::cache_list(session, query)?;
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
        Some(("remove", args)) => {
            if args.get_flag("all") {
                match operation::cache_remove(session, "*") {
                    Ok(_) => println!("All download caches were removed."),
                    Err(e) => return Err(e.into()),
                }
            }
            if let Some(query) = args.get_one::<String>("query").map(|s| s.as_ref()) {
                match operation::cache_remove(session, query) {
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
        _ => unreachable!(),
    }
}
