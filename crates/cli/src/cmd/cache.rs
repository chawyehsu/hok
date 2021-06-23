use clap::ArgMatches;

use scoop_core::{utils, Scoop};

pub fn cmd_cache(matches: &ArgMatches, scoop: &mut Scoop) {
    if let Some(sub_m2) = matches.subcommand_matches("rm") {
        if let Some(app_name) = sub_m2.value_of("app") {
            match scoop.cache_manager.remove(app_name) {
                Ok(()) => {
                    match app_name == "*" {
                        true => println!("All download caches were removed."),
                        false => println!("All caches that match '{}' were removed.", app_name),
                    }
                    std::process::exit(0);
                }
                Err(_e) => {
                    eprintln!("Failed to remove '{}' caches.", app_name);
                    std::process::exit(1);
                }
            }
        } else if sub_m2.is_present("all") {
            match scoop.cache_manager.remove_all() {
                Ok(()) => {
                    println!("All download caches were removed.");
                    std::process::exit(0);
                }
                Err(_e) => {
                    eprintln!("Failed to clear caches.");
                    std::process::exit(1);
                }
            }
        }
    } else {
        let cache_items = scoop.cache_manager.get_all().unwrap();
        let mut total_size: u64 = 0;
        let total_count = cache_items.len();

        if let Some(sub_m2) = matches.subcommand_matches("show") {
            if let Some(app) = sub_m2.value_of("app") {
                let mut filter_size: u64 = 0;
                let mut filter_count: u64 = 0;
                for sci in cache_items {
                    if sci.app_name().contains(app) {
                        filter_size = filter_size + sci.size();
                        filter_count = filter_count + 1;
                        println!(
                            "{: >6} {} ({}) {}",
                            utils::filesize(sci.size(), true),
                            sci.app_name(),
                            sci.version(),
                            sci.file_name()
                        );
                    }
                }
                if filter_count > 0 {
                    println!();
                }
                println!(
                    "Total: {} files, {}",
                    filter_count,
                    utils::filesize(filter_size, true)
                );
                std::process::exit(0);
            }
        }

        for sci in cache_items {
            total_size = total_size + sci.size();
            println!(
                "{: >6} {} ({}) {}",
                utils::filesize(sci.size(), true),
                sci.app_name(),
                sci.version(),
                sci.file_name()
            );
        }
        if total_count > 0 {
            println!();
        }
        println!(
            "Total: {} files, {}",
            total_count,
            utils::filesize(total_size, true)
        );
        std::process::exit(0);
    }
}
