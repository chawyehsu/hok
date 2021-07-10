use clap::ArgMatches;
use scoop_core::manager::CacheManager;
use scoop_core::util::filesize;
use scoop_core::Config;
use crate::error::CliResult;

pub fn cmd_cache(matches: &ArgMatches, config: &Config) -> CliResult<()> {
    let cache_manager = CacheManager::new(config);
    match matches.subcommand() {
        ("remove", Some(matches)) => {
            if let Some(app_name) = matches.value_of("app") {
                match cache_manager.remove(app_name) {
                    Ok(()) => {
                        if app_name == "*" {
                            println!("All download caches were removed.");
                        } else {
                            println!("All caches that match '{}' were removed.", app_name);
                        }
                        return Ok(());
                    }
                    Err(err) => return Err(err),
                }
            }
            if matches.is_present("all") {
                match cache_manager.remove_all() {
                    Ok(()) => {
                        println!("All download caches were removed.");
                        return Ok(());
                    }
                    Err(err) => return Err(err),
                }
            }
            Ok(())
        }
        ("list", Some(matches)) => {
            let cache_items = match matches.value_of("app") {
                Some(app_name) => cache_manager.entries_of(app_name).unwrap(),
                None => cache_manager.entries().unwrap(),
            };
            let mut total_size: u64 = 0;
            let total_count = cache_items.len();
            cache_items.into_iter().for_each(|file| {
                total_size += file.size();
                println!(
                    "{: >6} {} ({}) {:>}",
                    file.size_as_bytes(true),
                    file.app_name(),
                    file.version(),
                    file.filename()
                );
            });
            println!(
                "Total: {} files, {}", total_count, filesize(total_size, true)
            );
            Ok(())
        }
        _ => unreachable!(),
    }
}
