use crossterm::{
    style::{Color, Print, SetForegroundColor},
    ExecutableCommand,
};
use libscoop::Session;
use std::{fmt::Display, io};

mod clap_app;
mod cmd;
mod cui;
mod util;

type Result<T> = anyhow::Result<T>;

fn create_logger() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("HOK_LOG_LEVEL", "error")
        .write_style("never");
    match env_logger::try_init_from_env(env) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn error<T: Display>(input: &T) -> io::Result<()> {
    let mut stderr = io::stderr();
    stderr
        .execute(SetForegroundColor(Color::Red))?
        .execute(Print("ERROR "))?
        .execute(SetForegroundColor(Color::Reset))?
        .execute(Print(input))?
        .execute(Print("\n"))?;
    Ok(())
}

fn report(err: anyhow::Error) {
    let _ = error(&err);
    if let Some(cause) = err.source() {
        eprintln!("\nCaused by:");
        for (i, e) in std::iter::successors(Some(cause), |e| e.source()).enumerate() {
            eprintln!("   {}: {}", i, e);
        }
    }
}

pub fn create_app(args: Vec<String>) -> bool {
    let _ = create_logger();
    let app = clap_app::build();
    let session = Session::default();
    let _ = session.set_user_agent("Scoop/1.0 (+http://scoop.sh/) Hok/0.1.0");

    let ret = match app.try_get_matches_from(args) {
        Err(e) => {
            eprintln!("{}", e);
            Ok(())
        }
        Ok(matches) => match matches.subcommand() {
            Some(("bucket", matches)) => cmd::cmd_bucket(matches, &session),
            Some(("cache", matches)) => cmd::cmd_cache(matches, &session),
            Some(("cat", matches)) => cmd::cmd_cat(matches, &session),
            Some(("cleanup", matches)) => cmd::cmd_cleanup(matches, &session),
            Some(("config", matches)) => cmd::cmd_config(matches, &session),
            Some(("hold", matches)) => cmd::cmd_hold(matches, &session),
            Some(("home", matches)) => cmd::cmd_home(matches, &session),
            Some(("info", matches)) => cmd::cmd_info(matches, &session),
            Some(("install", matches)) => cmd::cmd_install(matches, &session),
            Some(("list", matches)) => cmd::cmd_list(matches, &session),
            Some(("search", matches)) => cmd::cmd_search(matches, &session),
            Some(("unhold", matches)) => cmd::cmd_unhold(matches, &session),
            Some(("uninstall", matches)) => cmd::cmd_uninstall(matches, &session),
            Some(("update", matches)) => cmd::cmd_update(matches, &session),
            Some(("upgrade", matches)) => cmd::cmd_upgrade(matches, &session),
            _ => unimplemented!(),
        },
    };

    match ret {
        Ok(_) => false,
        Err(e) => {
            report(e);
            true
        }
    }
}
