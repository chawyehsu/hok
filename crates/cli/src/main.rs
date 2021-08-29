use crate::error::CliResult;
use env_logger::Env;
use scoop_core::Config;

mod cli;
mod cmd;
mod console;
mod error;

fn create_logger() {
    let env = Env::default()
        .filter_or("SCOOP_LOG_LEVEL", "info")
        .write_style("never");

    env_logger::init_from_env(env);
}

fn run() -> CliResult<()> {
    create_logger();
    let app = cli::build_app();
    let matches = app.get_matches();
    // Init global config
    let mut config = Config::init();

    match matches.subcommand() {
        ("bucket", Some(matches)) => cmd::cmd_bucket(matches, &config)?,
        ("cache", Some(matches)) => cmd::cmd_cache(matches, &config)?,
        ("cleanup", Some(matches)) => cmd::cmd_cleanup(matches, &config),
        ("config", Some(matches)) => cmd::cmd_config(matches, &mut config)?,
        ("hold", Some(matches)) => cmd::cmd_hold(matches, &config),
        ("home", Some(matches)) => cmd::cmd_home(matches, &config)?,
        ("info", Some(matches)) => cmd::cmd_info(matches, &config)?,
        ("install", Some(matches)) => cmd::cmd_install(matches, &config)?,
        ("list", Some(matches)) => cmd::cmd_list(matches, &config)?,
        ("search", Some(matches)) => cmd::cmd_search(matches, &config)?,
        ("status", Some(matches)) => cmd::cmd_status(matches, &config),
        ("unhold", Some(matches)) => cmd::cmd_unhold(matches, &config),
        ("uninstall", Some(_matches)) => unimplemented!(),
        ("update", Some(matches)) => cmd::cmd_update(matches, &mut config),
        ("upgrade", Some(matches)) => cmd::cmd_upgrade(matches, &config),
        ("which", Some(_matches)) => unimplemented!(),
        _ => unreachable!(),
    }

    Ok(())
}

fn main() {
    let res = run();
    match res {
        Ok(()) => {}
        Err(e) => {
            drop(console::error(format!("{}", e)));
            std::process::exit(1);
        }
    }
}
