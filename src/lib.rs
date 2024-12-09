use crossterm::{
    style::{Color, Print, SetForegroundColor},
    ExecutableCommand,
};
use libscoop::Session;
use std::{fmt::Display, io};

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

pub fn create_app() -> bool {
    let _ = create_logger();
    let session = Session::default();
    let _ = session.set_user_agent("Scoop/1.0 (+http://scoop.sh/) Hok/0.1.0");

    match cmd::start(&session) {
        Ok(_) => false,
        Err(e) => {
            report(e);
            true
        }
    }
}
