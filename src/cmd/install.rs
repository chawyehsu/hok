#![allow(unused)]
use clap::ArgMatches;
use libscoop::{operation, Event, Session, SyncOption};
use std::{collections::HashSet, io::Write};

use crate::Result;

pub fn cmd_install(matches: &ArgMatches, session: &Session) -> Result<()> {
    let queries = matches
        .get_many::<String>("package")
        .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut options = vec![];

    if matches.get_flag("download-only") {
        options.push(SyncOption::DownloadOnly);
    }

    if matches.get_flag("ignore-cache") {
        options.push(SyncOption::IgnoreCache);
    }

    let rx = session.event_bus().receiver();
    let tx = session.event_bus().sender();

    let handle = std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            match event {
                Event::PackageResolveStart => println!("Resolving packages..."),
                Event::PackageDownloadSizingStart => {
                    println!("Calculating download size...");
                }
                Event::SelectPackage(pkgs) => {
                    let name = pkgs[0].split_once('/').unwrap().1;
                    println!("Found multiple candidates for package '{}':\n", name);
                    for (i, pkg) in pkgs.iter().enumerate() {
                        println!("  {}: {}", i, pkg);
                    }

                    let mut index = 0usize;
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
                        println!("Invalid input.");
                    }

                    tx.send(Event::SelectPackageAnswer(index));
                }
                _ => {}
            }
        }
    });

    operation::package_sync(session, queries, options)?;
    eprintln!("Not fully implemented yet.");
    Ok(())
}
