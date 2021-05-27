use clap::ArgMatches;

use crate::Scoop;

pub fn cmd_config(matches: &ArgMatches, scoop: &mut Scoop) {
  if matches.is_present("edit") {
    unimplemented!();
  } else if matches.is_present("list") {
    for (key, value) in scoop.config.get_all().as_object().unwrap() {
      println!("{}={}", key, value);
    }
  } else if matches.is_present("set") {
    let vals: Vec<&str> = matches.values_of("set").unwrap().collect();
    scoop.config.set(vals[0], vals[1]).save();
  } else if matches.is_present("unset") {
    let key = matches.value_of("unset").unwrap();
    scoop.config.remove(key).save();
    for (key, value) in scoop.config.get_all().as_object().unwrap() {
      println!("{}={}", key, value);
    }
  }
}
