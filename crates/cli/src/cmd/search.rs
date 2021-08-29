use clap::ArgMatches;
use scoop_core::ops::search;
use scoop_core::Config;

use crate::error::CliResult;

pub fn cmd_search(matches: &ArgMatches, config: &Config) -> CliResult<()> {
    if let Some(query) = matches.value_of("query") {
        for (bucket, apps) in search(config, query)? {
            println!("'{}' bucket:", bucket);
            for (app, bin) in apps {
                let name = app.name();
                let version = app.manifest().version();
                let prt = match bin {
                    Some(s) => format!("    {} ({}) --> includes '{}'", name, version, s),
                    None => format!("    {} ({})", name, version),
                };
                println!("{}", prt);
            }
            println!("");
        }
    }
    Ok(())
}
