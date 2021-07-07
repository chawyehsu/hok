use clap::ArgMatches;
use scoop_core::Config;

pub fn cmd_config(matches: &ArgMatches, config: &mut Config) {
    if matches.is_present("edit") {
        unimplemented!();
    } else if matches.is_present("list") {
        println!("{}", config);
    } else if matches.is_present("set") {
        let vals: Vec<&str> = matches.values_of("set").unwrap().collect();
        match config.set(vals[0], vals[1]) {
            Ok(cfg) => cfg.save(),
            Err(err) => eprintln!("{}", err),
        }
    } else if matches.is_present("unset") {
        let key = matches.value_of("unset").unwrap();
        match config.unset(key) {
            Ok(cfg) => cfg.save(),
            Err(err) => eprintln!("{}", err),
        }
    }
}
