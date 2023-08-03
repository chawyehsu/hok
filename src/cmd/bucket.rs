use clap::ArgMatches;
use crossterm::style::Stylize;
use libscoop::{operation, Session};
use std::io::{stdout, Write};

use crate::Result;

pub fn cmd_bucket(matches: &ArgMatches, session: &Session) -> Result<()> {
    match matches.subcommand() {
        Some(("add", args)) => {
            let name = args
                .get_one::<String>("name")
                .map(|s| s.as_str())
                .unwrap_or_default();
            let repo = args
                .get_one::<String>("repo")
                .map(|s| s.as_str())
                .unwrap_or_default();
            print!("Adding bucket {}... ", name);
            let _ = stdout().flush();
            match operation::bucket_add(session, name, repo) {
                Ok(..) => println!("{}", "Ok".green()),
                Err(err) => {
                    println!("{}", "Err".red());
                    return Err(err.into());
                }
            }
            Ok(())
        }
        Some(("list", args)) => {
            let known = args.get_flag("known");
            if known {
                for (name, repo) in operation::bucket_list_known() {
                    println!("{} {}", name.green(), repo);
                }
                Ok(())
            } else {
                match operation::bucket_list(session) {
                    Err(e) => Err(e.into()),
                    Ok(buckets) => {
                        for bucket in buckets {
                            println!(
                                "{}\n ├─manifests: {}\n └─source: {}",
                                bucket.name().green(),
                                bucket.manifest_count(),
                                bucket.source(),
                            );
                        }
                        Ok(())
                    }
                }
            }
        }
        Some(("remove", args)) => {
            let names = args
                .get_many::<String>("name")
                .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            for name in names {
                print!("Removing bucket {}... ", name);
                let _ = stdout().flush();
                match operation::bucket_remove(session, name) {
                    Ok(..) => println!("{}", "Ok".green()),
                    Err(err) => {
                        println!("{}", "Err".red());
                        return Err(err.into());
                    }
                }
            }
            Ok(())
        }
        _ => unreachable!(),
    }
}
