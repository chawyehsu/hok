#![allow(unused)]
use clap::ArgMatches;
use crossterm::style::Stylize;
use libscoop::{operation, Session};
use std::{
    io::{stdout, Write},
    path::Path,
    result,
};

use crate::Result;

pub fn cmd_cleanup(matches: &ArgMatches, session: &Session) -> Result<()> {
    let config = session.config();
    let apps_path = config.root_path().join("apps");
    // let running_apps = running_apps(&apps_path);

    let query = matches
        .get_many::<String>("app")
        .map(|v| v.map(|s| s.as_str()).collect::<Vec<_>>())
        .unwrap_or_default()
        .join(" ");

    eprintln!("Not implemented yet.");

    // for package in packages {
    //     let name = package.name.as_str();
    //     let package_path = apps_path.join(name);
    //     let current_version = package.version();
    //     let entries = std::fs::read_dir(&package_path)?
    //         .filter_map(result::Result::ok)
    //         .filter(|e| {
    //             let cur_version = e
    //                 .file_name()
    //                 .to_str()
    //                 .map(|s| s != current_version)
    //                 .unwrap_or(false);
    //             let current_symlink = e
    //                 .file_name()
    //                 .to_str()
    //                 .map(|s| s == "current")
    //                 .unwrap_or(false);
    //             !cur_version && !current_symlink
    //         })
    //         .collect::<Vec<_>>();

    //     if entries.is_empty() {
    //         continue;
    //     }

    //     print!("Cleaning up {}... ", name);
    //     let _ = stdout().flush();
    //     for entry in entries {
    //         let entry_name = entry.file_name();
    //         let entry_name = entry_name.to_str().unwrap_or_default();
    //         if entry.file_type().unwrap().is_dir() {
    //             print!("{}{} ", entry_name, "✓".green());
    //             let _ = stdout().flush();
    //         }

    //         remove_dir_all::remove_dir_all(entry.path())?;
    //     }
    //     println!("");
    // }

    // println!("{}", "Everything is shiny now!".green());

    Ok(())
}
