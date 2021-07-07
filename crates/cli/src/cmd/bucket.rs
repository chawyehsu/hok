use clap::ArgMatches;
use scoop_core::{BucketManager, Config};

use crate::error::CliResult;

pub fn cmd_bucket(matches: &ArgMatches, config: &Config) -> CliResult<()> {
    let bucket_manager = BucketManager::new(config);

    match matches.subcommand() {
        ("add", Some(matches)) => {
            let name = matches.value_of("name").unwrap();
            let repo = matches.value_of("repo");
            return bucket_manager.add_bucket(name, repo);
        }
        ("list", Some(_)) => {
            bucket_manager.buckets().iter().for_each(|(name, _)| {
                println!("{}", name);
            });
        }
        ("known", Some(_)) => {
            bucket_manager.known_buckets().iter().for_each(|name| {
                println!("{}", name);
            });
        }
        ("remove", Some(matches)) => {
            let name = matches.value_of("name").unwrap();
            return bucket_manager.remove_bucket(name);
        }
        _ => unreachable!(),
    }

    Ok(())
}
