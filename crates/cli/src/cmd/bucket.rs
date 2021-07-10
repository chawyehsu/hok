use clap::ArgMatches;
use scoop_core::{manager::BucketManager, Config};

use crate::error::CliResult;

pub fn cmd_bucket(matches: &ArgMatches, config: &Config) -> CliResult<()> {
    let bucket_manager = BucketManager::new(config);
    match matches.subcommand() {
        ("add", Some(matches)) => {
            let name = matches.value_of("name").unwrap();
            let repo = matches.value_of("repo");
            match bucket_manager.add_bucket(name, repo) {
                Ok(()) => {
                    println!("The {} bucket was added successfully", name);
                    return Ok(());
                }
                Err(e) => return Err(e),
            }
        }
        ("list", Some(_)) => {
            bucket_manager.buckets().iter().for_each(|bucket| {
                println!("{}", bucket.name());
            });
        }
        ("known", Some(_)) => {
            bucket_manager.known_buckets().iter().for_each(|name| {
                println!("{}", name);
            });
        }
        ("remove", Some(matches)) => {
            let name = matches.value_of("name").unwrap();
            match bucket_manager.remove_bucket(name) {
                Ok(()) => {
                    println!("The {} bucket was removed successfully", name);
                    return Ok(());
                }
                Err(e) => return Err(e),
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
