extern crate anyhow;
extern crate remove_dir_all;

use anyhow::Result;
use scoop::{Scoop, cli, cmd, config::Config};
use env_logger::Env;

fn main() -> Result<()> {
  // logger
  let env = Env::default()
    .filter_or("SCOOP_LOG_LEVEL", "trace")
    .write_style("never");
  env_logger::init_from_env(env);

  let app = cli::build_app();
  let matches = app.get_matches();
  let config = Config::new();
  let mut scoop = Scoop::new(config);

  match matches.subcommand() {
    ("bucket", Some(matches)) => cmd::cmd_bucket(matches, &mut scoop),
    ("cache", Some(matches)) => cmd::cmd_cache(matches, &mut scoop),
    ("config", Some(matches)) => cmd::cmd_config(matches, &mut scoop),
    ("home", Some(matches)) => cmd::cmd_home(matches, &mut scoop),
    ("info", Some(matches)) => cmd::cmd_info(matches, &mut scoop),
    ("install", Some(_matches)) => unimplemented!(),
    ("list", Some(matches)) => cmd::cmd_list(matches, &mut scoop),
    ("search", Some(matches)) => cmd::cmd_search(matches, &mut scoop),
    ("update", Some(matches)) => cmd::cmd_update(matches, &mut scoop),
    _ => unreachable!(),
  }

  Ok(())
}
