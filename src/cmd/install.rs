#![allow(unused_assignments)]
use clap::ArgMatches;
use crossterm::{
    cursor,
    style::Stylize,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use libscoop::{operation, Event, Session, SyncOption};
use std::io::Write;

use crate::{cui, util, Result};

pub fn cmd_install(matches: &ArgMatches, session: &Session) -> Result<()> {
    let mut options = vec![];
    let queries = matches
        .get_many::<String>("package")
        .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    if matches.get_flag("assume-yes") {
        options.push(SyncOption::AssumeYes);
    }

    if matches.get_flag("download-only") {
        options.push(SyncOption::DownloadOnly);
    }

    if matches.get_flag("escape-hold") {
        options.push(SyncOption::EscapeHold);
    }

    if matches.get_flag("ignore-failure") {
        options.push(SyncOption::IgnoreFailure);
    }

    if matches.get_flag("ignore-cache") {
        options.push(SyncOption::IgnoreCache);
    }

    if matches.get_flag("no-upgrade") {
        options.push(SyncOption::NoUpgrade);
    }

    if matches.get_flag("no-replace") {
        options.push(SyncOption::NoReplace);
    }

    if matches.get_flag("offline") {
        options.push(SyncOption::Offline);
    }

    if matches.get_flag("independent") {
        options.push(SyncOption::NoDependencies);
    }

    if matches.get_flag("no-hash-check") {
        options.push(SyncOption::NoHashCheck);
    }

    let rx = session.event_bus().receiver();
    let tx = session.event_bus().sender();

    let mut stdout = std::io::stdout();
    let _ = stdout.execute(cursor::Hide);

    let mut dlprogress = cui::MultiProgressUI::new();

    let handle = std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            match event {
                Event::PackageResolveStart => println!("Resolving packages..."),
                Event::PackageDownloadSizingStart => println!("Calculating download size..."),
                Event::PackageDownloadStart => println!("Downloading packages..."),
                Event::PackageDownloadProgress(ctx) => {
                    let ident = ctx.ident.to_owned();
                    let url = ctx.url.to_owned();
                    let filename = ctx.filename.to_owned();
                    let dltotal = ctx.dltotal;
                    let dlnow = ctx.dlnow;

                    dlprogress.update(ident, url, filename, dltotal, dlnow);
                }
                Event::PackageDownloadDone => {}
                Event::PackageIntegrityCheckStart => println!("Checking package integrity..."),
                Event::PackageIntegrityCheckProgress(ctx) => {
                    let mut stdout = std::io::stdout();
                    stdout
                        .execute(cursor::MoveToPreviousLine(1))
                        .unwrap()
                        .execute(Clear(ClearType::CurrentLine))
                        .unwrap();
                    println!("Checking package integrity...{}", ctx.dark_grey());
                }
                Event::PackageIntegrityCheckDone => {
                    let mut stdout = std::io::stdout();
                    stdout
                        .execute(cursor::MoveToPreviousLine(1))
                        .unwrap()
                        .execute(Clear(ClearType::CurrentLine))
                        .unwrap();
                    println!("Checking package integrity...{}", "Ok".green());
                }
                Event::PromptPackageCandidate(pkgs) => {
                    let name = pkgs[0].split_once('/').unwrap().1;
                    println!("Found multiple candidates for package '{}':\n", name);
                    for (i, pkg) in pkgs.iter().enumerate() {
                        println!("  {}: {}", i, pkg);
                    }

                    let mut index = 0;
                    let mut stdout = std::io::stdout();
                    let _ = stdout.execute(cursor::Show);
                    loop {
                        print!("\nPlease select one, enter the number to continue: ");
                        std::io::stdout().flush().unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        let parsed = input.trim().parse::<usize>();
                        if let Ok(num) = parsed {
                            index = num;
                            // bounds check
                            if num < pkgs.len() {
                                break;
                            }
                        }
                    }

                    let _ = stdout.execute(cursor::Hide);
                    let _ = tx.send(Event::PromptPackageCandidateResult(index));
                }
                Event::PromptTransactionNeedConfirm(transaction) => {
                    if let Some(install) = transaction.install_view() {
                        println!("The following packages will be INSTALLED:");
                        let output = install
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

                    if let Some(upgrade) = transaction.upgrade_view() {
                        if transaction.install_view().is_some() {
                            println!();
                        }
                        println!("The following packages will be UPGRADED:");
                        let output = upgrade
                            .iter()
                            .map(|p| {
                                format!(
                                    "{}{}{}",
                                    p.ident(),
                                    "-".dark_grey(),
                                    p.upgradable_version().unwrap().dark_grey(),
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("  ");
                        println!("  {}", output);
                    }

                    if let Some(replace) = transaction.replace_view() {
                        if transaction.install_view().is_some()
                            || transaction.upgrade_view().is_some()
                        {
                            println!();
                        }
                        println!("The following packages will be REPLACED:");
                        let output = replace
                            .iter()
                            .map(|p| {
                                format!(
                                    "{}{}/{}",
                                    p.installed_bucket().unwrap().dark_grey().crossed_out(),
                                    p.bucket(),
                                    p.name(),
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("  ");
                        println!("  {}", output);
                    }

                    if let Some(download_size) = transaction.download_size() {
                        let out = util::humansize(download_size.total, true);
                        if download_size.total > 0 {
                            if download_size.estimated {
                                println!(
                                    "\nTotal download size: {} {}",
                                    out,
                                    "(estimated)".dark_grey()
                                );
                            } else {
                                println!("\nTotal download size: {}", out);
                            }
                        } else {
                            println!("\nNothing to download, all cached.");
                        }
                    }

                    let mut stdout = std::io::stdout();
                    let _ = stdout.execute(cursor::Show);
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

                    let _ = stdout.execute(cursor::Hide);
                }
                Event::PackageSyncDone => break,
                _ => {}
            }
        }
    });

    operation::package_sync(session, queries, options)?;

    handle.join().unwrap();

    let mut stdout = std::io::stdout();
    let _ = stdout.execute(cursor::Show);

    eprintln!("Not implemented yet.");
    Ok(())
}
