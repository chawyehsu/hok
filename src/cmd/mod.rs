use clap::{crate_description, crate_name, crate_version, Parser, Subcommand};

mod bucket;
mod cache;
mod cat;
mod cleanup;
mod completions;
mod config;
mod hold;
mod home;
mod info;
mod install;
mod list;
mod search;
mod unhold;
mod uninstall;
mod update;
mod upgrade;

use crate::Result;
use libscoop::Session;

#[derive(Parser)]
#[command(
    name = crate_name!(),
    version = crate_version!(),
    about = crate_description!(),
    subcommand_required = true,
    arg_required_else_help = true,
    max_term_width = 100,
    after_help = format!(
        "Type '{} help <command>' to get help for a specific command.",
        crate_name!()
    )
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Bucket(bucket::Args),
    Cache(cache::Args),
    Cat(cat::Args),
    Cleanup(cleanup::Args),
    Completions(completions::Args),
    Config(config::Args),
    Hold(hold::Args),
    Home(home::Args),
    Info(info::Args),
    #[clap(alias = "i")]
    Install(install::Args),
    List(list::Args),
    #[clap(alias = "s")]
    Search(search::Args),
    Unhold(unhold::Args),
    #[clap(alias = "rm", alias = "remove")]
    Uninstall(uninstall::Args),
    #[clap(alias = "u")]
    Update(update::Args),
    Upgrade(upgrade::Args),
}

/// CLI entry point
pub fn start(session: &Session) -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Command::Bucket(args) => bucket::execute(args, session),
        Command::Cache(args) => cache::execute(args, session),
        Command::Cat(args) => cat::execute(args, session),
        Command::Cleanup(args) => cleanup::execute(args, session),
        Command::Completions(args) => completions::execute(args),
        Command::Config(args) => config::execute(args, session),
        Command::Hold(args) => hold::execute(args, session),
        Command::Home(args) => home::execute(args, session),
        Command::Info(args) => info::execute(args, session),
        Command::Install(args) => install::execute(args, session),
        Command::List(args) => list::execute(args, session),
        Command::Search(args) => search::execute(args, session),
        Command::Unhold(args) => unhold::execute(args, session),
        Command::Uninstall(args) => uninstall::execute(args, session),
        Command::Update(args) => update::execute(args, session),
        Command::Upgrade(args) => upgrade::execute(args, session),
    }
}
