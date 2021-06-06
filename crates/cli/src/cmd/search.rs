use clap::ArgMatches;

use crate::Scoop;

pub fn cmd_search(matches: &ArgMatches, scoop: &mut Scoop) {
    if let Some(query) = matches.value_of("query") {
        let search_bin = matches.is_present("binary");
        let matches = scoop.search(query, search_bin).unwrap();

        for m in matches {
            if m.collected.len() > 0 {
                println!("'{}' bucket:", m.bucket);
                for sm in m.collected {
                    if sm.bin.is_none() {
                        println!("  {} ({})", sm.name, sm.version);
                    } else {
                        println!(
                            "  {} ({}) --> includes {}",
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
