use clap::ArgMatches;

use crate::Scoop;

pub fn cmd_search(matches: &ArgMatches, scoop: &mut Scoop) {
  if let Some(query) = matches.value_of("query") {
    let fuzzy = matches.is_present("fuzzy");
    let with_binary = matches.is_present("binary");
    scoop.search(query, fuzzy, with_binary).unwrap();
  }
}
