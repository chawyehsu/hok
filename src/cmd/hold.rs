use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use libscoop::{operation, Session};

use crate::Result;

/// Hold package(s) to disable changes
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package(s) to be held
    #[arg(required= true, action = ArgAction::Append)]
    package: Vec<String>,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    for name in &args.package {
        print!("Holding {}...", name);
        match operation::package_hold(session, name, true) {
            Ok(..) => {
                println!("{}", "Ok".green());
            }
            Err(err) => {
                println!("{}", "Err".red());
                return Err(err.into());
            }
        }
    }
    Ok(())
}
