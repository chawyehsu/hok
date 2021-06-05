use clap::ArgMatches;

use crate::Scoop;

pub fn cmd_config(matches: &ArgMatches, scoop: &mut Scoop) {
    if matches.is_present("edit") {
        unimplemented!();
    } else if matches.is_present("list") {
        println!("{}", scoop.config);
    } else if matches.is_present("set") {
        let vals: Vec<&str> = matches.values_of("set").unwrap().collect();
        match scoop.config.set(vals[0], vals[1]) {
            Ok(cfg) => cfg.save(),
            Err(err) => eprintln!("{}", err),
        }
    } else if matches.is_present("unset") {
        let key = matches.value_of("unset").unwrap();
        match scoop.config.unset(key) {
            Ok(cfg) => cfg.save(),
            Err(err) => eprintln!("{}", err),
        }
    }
}
