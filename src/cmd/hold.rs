use clap::ArgMatches;

use crate::Scoop;

pub fn cmd_hold(matches: &ArgMatches, scoop: &mut Scoop) {
  if let Some(name) = matches.value_of("app") {
    if scoop.app_manager.is_app_installed(name) {
      unimplemented!();
    }
  }
}
