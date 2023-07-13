use env_logger::Env;
use scoop_core::Session;

mod cli;
mod cmd;
mod console;

pub type Result<T> = anyhow::Result<T>;

fn create_logger() -> Result<()> {
    let env = Env::default()
        .filter_or("SCOOP_LOG_LEVEL", "info")
        .write_style("never");
    match env_logger::try_init_from_env(env) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn report(err: anyhow::Error) {
    let _ = console::error(&err);
    if let Some(cause) = err.source() {
        eprintln!("\nCaused by:");
        for (i, e) in std::iter::successors(Some(cause), |e| e.source()).enumerate() {
            eprintln!("   {}: {}", i, e);
        }
    }
}

pub fn create_app(args: Vec<String>) -> bool {
    let _ = create_logger();
    let app = cli::build();
    let mut session = Session::init().unwrap();
    let ret = match app.get_matches_from_safe(args) {
        Err(e) => {
            eprintln!("{}", e);
            Ok(())
        }
        Ok(matches) => {
            match matches.subcommand() {
                ("bucket", Some(matches)) => cmd::cmd_bucket(matches, &session),
                ("cache", Some(matches)) => cmd::cmd_cache(matches, &session),
                ("cat", Some(matches)) => cmd::cmd_cat(matches, &session),
                ("cleanup", Some(matches)) => cmd::cmd_cleanup(matches, &session),
                ("config", Some(matches)) => cmd::cmd_config(matches, &mut session),
                ("hold", Some(matches)) => cmd::cmd_hold(matches, &session),
                ("home", Some(matches)) => cmd::cmd_home(matches, &session),
                ("info", Some(matches)) => cmd::cmd_info(matches, &session),
                ("install", Some(matches)) => cmd::cmd_install(matches, &session),
                ("list", Some(matches)) => cmd::cmd_list(matches, &session),
                ("search", Some(matches)) => cmd::cmd_search(matches, &session),
                ("unhold", Some(matches)) => cmd::cmd_unhold(matches, &session),
                // ("uninstall", Some(_matches)) => unimplemented!(),
                ("update", Some(matches)) => cmd::cmd_update(matches, &mut session),
                ("upgrade", Some(matches)) => cmd::cmd_upgrade(matches, &session),
                // ("which", Some(_matches)) => unimplemented!(),
                _ => unreachable!(),
            }
        }
    };

    match ret {
        Ok(_) => false,
        Err(e) => {
            report(e);
            true
        }
    }
}
