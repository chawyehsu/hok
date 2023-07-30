use clap::ArgMatches;
use crossterm::style::Stylize;
use libscoop::{operation, Event, Session, SyncOption};
use std::io::Write;

use crate::Result;

pub fn cmd_uninstall(matches: &ArgMatches, session: &Session) -> Result<()> {
    let queries = matches
        .get_many::<String>("package")
        .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut options = vec![SyncOption::Remove];

    if matches.get_flag("assume-yes") {
        options.push(SyncOption::AssumeYes);
    }

    if matches.get_flag("cascade") {
        options.push(SyncOption::Cascade);
    }

    if matches.get_flag("no-dependent-check") {
        options.push(SyncOption::NoDependentCheck);
    }

    if matches.get_flag("escape-hold") {
        options.push(SyncOption::EscapeHold);
    }

    if matches.get_flag("purge") {
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

                    loop {
                        print!("\nDo you want to continue? [y/N]: ");
                        std::io::stdout().flush().unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        //
                        if input.chars().count() == 3 {
                            let ch: char = input.chars().next().unwrap();
                            if ['y', 'Y', 'n', 'N'].contains(&ch) {
                                let ret = ch == 'y' || ch == 'Y';
                                let _ = tx.send(Event::PromptTransactionNeedConfirmResult(ret));
                                break;
                            }
                        }
                    }
                }
                Event::PackageSyncDone => break,
                _ => {}
            }
        }
    });

    operation::package_sync(session, queries, options)?;

    handle.join().unwrap();

    eprintln!("Not implemented yet.");
    Ok(())
}
