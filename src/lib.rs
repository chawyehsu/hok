use crossterm::{
    style::{Color, Print, SetForegroundColor},
    ExecutableCommand,
};
use std::{fmt::Display, io};

mod cmd;
mod cui;
mod util;

type Result<T> = anyhow::Result<T>;

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

fn report(err: &anyhow::Error) {
    let _ = error(err);
    if let Some(cause) = err.source() {
        eprintln!("\nCaused by:");
        for (i, e) in std::iter::successors(Some(cause), |e| e.source()).enumerate() {
            eprintln!("   {}: {}", i, e);
        }
    }
}

pub fn create_app() -> bool {
    cmd::start().inspect_err(report).is_err()
}
