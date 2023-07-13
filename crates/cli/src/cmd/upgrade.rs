use clap::ArgMatches;
use scoop_core::Session;

use crate::Result;

pub fn cmd_upgrade(matches: &ArgMatches, session: &Session) -> Result<()> {
    let query = matches
        .values_of("package")
        .map(|v| v.collect::<Vec<_>>())
        .unwrap_or_default()
        .join(" ");
    let flag_upgradable = true;
    match session.package_list(&query, flag_upgradable) {
        Err(e) => Err(e.into()),
        Ok(packages) => {
            // for pkg in packages {
            //     let name = pkg.name.as_str();
            //     let bucket = pkg.bucket.as_str();
            //     let version = pkg.version.as_str();
            //     println!("{}/{} {}", name, bucket, version)
            // }
            Ok(())
        }
    }
}
