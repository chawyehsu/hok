use clap::{crate_name, CommandFactory, Parser};
use clap_complete::Shell;

use crate::{cmd::Cli, Result};

/// Generate shell completions
#[derive(Parser, Debug)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The shell type
    shell: Shell,
}

pub fn execute(args: Args) -> Result<()> {
    let mut buf = vec![];
    clap_complete::generate(args.shell, &mut Cli::command(), crate_name!(), &mut buf);
    println!(
        "{}",
        String::from_utf8(buf).expect("clap_complete did not generate valid shell script")
    );
    Ok(())
}
