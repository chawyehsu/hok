use clap::ArgMatches;
use scoop_core::{error::Error, Session};
use std::process::Command;

use crate::Result;

pub fn cmd_home(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(query) = matches.value_of("package") {
        let mode = "unique";
        match session.package_search(query, mode) {
            Err(Error::PackageNotFound { .. }) => {
                eprintln!("Could not find package named '{}'.", query);
            }
            Err(Error::PackageMultipleRecordsFound { records }) => {
                let packages = &records[0].1;
                eprintln!("Found multiple packages named '{}':\n", query);
                for (idx, pkg) in packages.iter().enumerate() {
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
                let url = std::ffi::OsStr::new(package.homepage());
                Command::new("cmd")
                    .arg("/C")
                    .arg("start")
                    .arg(url)
                    .spawn()?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
