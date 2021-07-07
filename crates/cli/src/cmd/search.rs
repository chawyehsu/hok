use clap::ArgMatches;
use scoop_core::{search, Config};

pub fn cmd_search(matches: &ArgMatches, config: &Config) {
    if let Some(query) = matches.value_of("query") {
        let search_bin = matches.is_present("binary");
        let matches = search(config, query, search_bin).unwrap();

        for m in matches {
            if m.collected.len() > 0 {
                println!("'{}' bucket:", m.bucket);
                for sm in m.collected {
                    if sm.bin.is_none() {
                        println!("    {} ({})", sm.name, sm.version);
                    } else {
                        println!(
                            "    {} ({}) --> includes {}",
                            sm.name,
                            sm.version,
                            sm.bin.unwrap()
                        );
                    }
                }
                println!("");
            }
        }
    }
}
