use anyhow::Result;
use scoop::{Scoop, cli, cmd, config::Config, log::create_logger};

fn main() -> Result<()> {
  create_logger();
  let app = cli::build_app();
  let matches = app.get_matches();
  // Init global config
  let mut config = Config::new();
  // Create scoop instance via global config
  let mut scoop = Scoop::new(&mut config);

  match matches.subcommand() {
    ("bucket", Some(matches)) => cmd::cmd_bucket(matches, &mut scoop),
    ("cache", Some(matches)) => cmd::cmd_cache(matches, &mut scoop),
    ("cleanup", Some(matches)) => cmd::cmd_cleanup(matches, &mut scoop),
    ("config", Some(matches)) => cmd::cmd_config(matches, &mut scoop),
    ("hold", Some(_matches)) => unimplemented!(),
    ("home", Some(matches)) => cmd::cmd_home(matches, &mut scoop),
    ("info", Some(matches)) => cmd::cmd_info(matches, &mut scoop),
    ("install", Some(_matches)) => unimplemented!(),
    ("list", Some(matches)) => cmd::cmd_list(matches, &mut scoop),
    ("search", Some(matches)) => cmd::cmd_search(matches, &mut scoop),
    ("status", Some(_matches)) => unimplemented!(),
    ("unhold", Some(_matches)) => unimplemented!(),
    ("uninstall", Some(_matches)) => unimplemented!(),
    ("update", Some(matches)) => cmd::cmd_update(matches, &mut scoop),
    ("which", Some(_matches)) => unimplemented!(),
    _ => unreachable!(),
  }

  Ok(())
}
