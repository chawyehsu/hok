use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use libscoop::{operation, Event, Session, SyncOption};

use crate::{cui, Result};

/// Uninstall package(s)
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package(s) to uninstall
    #[arg(required = true, action = ArgAction::Append)]
    package: Vec<String>,
    /// Remove unneeded dependencies as well
    #[arg(short = 'c', long, action = ArgAction::SetTrue)]
    cascade: bool,
    /// Purge package(s) persistent data as well
    #[arg(short = 'p', long, action = ArgAction::SetTrue)]
    purge: bool,
    /// Assume yes to all prompts and run non-interactively
    #[arg(short = 'y', long, action = ArgAction::SetTrue)]
    assume_yes: bool,
    /// Disable dependent check (may break other packages)
    #[arg(long, action = ArgAction::SetTrue)]
    no_dependent_check: bool,
    /// Escape hold to allow to uninstall held package(s)
    #[arg(short = 'S', long, action = ArgAction::SetTrue)]
    escape_hold: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let queries = args.package.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let mut options = vec![SyncOption::Remove];

    if args.assume_yes {
        options.push(SyncOption::AssumeYes);
    }

    if args.cascade {
        options.push(SyncOption::Cascade);
    }

    if args.no_dependent_check {
        options.push(SyncOption::NoDependentCheck);
    }

    if args.escape_hold {
        options.push(SyncOption::EscapeHold);
    }

    if args.purge {
        options.push(SyncOption::Purge);
    }

    let rx = session.event_bus().receiver();
    let tx = session.event_bus().sender();

    let handle = std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            match event {
                Event::PackageResolveStart => println!("Resolving packages..."),
                Event::PromptTransactionNeedConfirm(transaction) => {
                    if let Some(remove) = transaction.remove_view() {
                        println!("The following packages will be REMOVED:");
                        let output = remove
                            .iter()
                            .map(|p| {
                                format!(
                                    "{}{}{}",
                                    p.ident(),
                                    "-".dark_grey(),
                                    p.version().dark_grey(),
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("  ");
                        println!("  {}", output);
                    }

                    let answer = cui::prompt_yes_no();
                    let _ = tx.send(Event::PromptTransactionNeedConfirmResult(answer));
                }
                Event::PackageCommitStart(ctx) => {
                    println!("Uninstalling {}...", ctx);
                }
                Event::PackageShortcutRemoveProgress(ctx) => {
                    println!("Removing shortcut {}", ctx);
                }
                Event::PackageShimRemoveProgress(ctx) => {
                    println!("Removing shim '{}'", ctx);
                }
                Event::PackagePersistPurgeStart => {
                    println!("Removing persisted data...");
                }
                Event::PackageCommitDone(ctx) => {
                    let msg = format!("'{}' was uninstalled.", ctx);
                    println!("{}", msg.dark_green());
                }
                Event::PackageSyncDone => break,
                _ => {}
            }
        }
    });

    operation::package_sync(session, queries, options)?;
    handle.join().unwrap();

    Ok(())
}
