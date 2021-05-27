use clap::ArgMatches;

use crate::Scoop;

pub fn cmd_update(_matches: &ArgMatches, scoop: &mut Scoop) {
  scoop.update_buckets().unwrap();
}
