use std::process::Command;
use clap::ArgMatches;

use crate::Scoop;

pub fn cmd_home(matches: &ArgMatches, scoop: &mut Scoop) {
  if let Some(app_name) = matches.value_of("app") {
    // find local manifest and parse it
    match scoop.find_local_manifest(app_name).unwrap() {
      Some(manifest) => {
        if let Some(url) = manifest.json.get("homepage") {
          let url = std::ffi::OsStr::new(url.as_str().unwrap());
          Command::new("cmd").arg("/C").arg("start").arg(url).spawn().unwrap();
        } else {
          println!("Could not find homepage in manifest for '{}'.", app_name);
        }
      },
      None => {
        println!("Could not find manifest for '{}'.", app_name);
      }
    }
  }
}
