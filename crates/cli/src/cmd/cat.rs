use clap::ArgMatches;
use scoop_core::{error::Error, Session};
use std::{path::Path, process::Command};

use crate::Result;

pub fn cmd_cat(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(query) = matches.value_of("package") {
        let mode = "unique";
        let result = session.package_search(query, mode);

        match result {
            Err(Error::PackageNotFound { .. }) => {
                eprintln!("Could not find package named '{}'.", query);
            }
            Err(Error::PackageMultipleRecordsFound { records }) => {
                let result = &records[0].1;

                eprintln!("Found multiple packages named '{}':\n", query);
                for (idx, pkg) in result.iter().enumerate() {
                    println!(
                        "  {}. {}/{} ({})",
                        idx + 1,
                        pkg.bucket,
                        pkg.name,
                        pkg.homepage()
                    );
                }
                eprintln!("\nUse more specific package name to narrow results.");
            }
            Ok(packages) => {
                let package = &packages[0];
                let cat = match is_program_available("bat.exe") {
                    true => "bat.exe",
                    false => "type",
                };
                let cat_args = match cat == "bat.exe" {
                    false => vec![],
                    true => {
                        let cat_style = session.config.cat_style();
                        vec!["--no-paging", "--style", cat_style, "--language", "json"]
                    }
                };

                let mut child = Command::new("cmd")
                    .arg("/C")
                    .arg(cat)
                    .arg(&package.manifest_path)
                    .args(cat_args)
                    .spawn()?;
                child.wait()?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Check if a given executable is available on the system
fn is_program_available(exe: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for p in path.split(";") {
            let path = Path::new(p).join(exe);
            if std::fs::metadata(path).is_ok() {
                return true;
            }
        }
    }
    false
}
