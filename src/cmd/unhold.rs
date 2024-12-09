use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use libscoop::{operation, Session};

use crate::Result;

/// Unhold package(s) to enable changes
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package(s) to be unheld
    #[arg(required = true, action = ArgAction::Append)]
    package: Vec<String>,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let packages = args.package.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    for name in packages {
        print!("Unholding {}...", name);
        match operation::package_hold(session, name, false) {
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
