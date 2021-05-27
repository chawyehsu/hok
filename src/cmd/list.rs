use clap::ArgMatches;

use crate::Scoop;

pub fn cmd_list(_matches: &ArgMatches, scoop: &mut Scoop) {
  let brew_list_mode = false; // scoop.config.brew_mode
      // .unwrap_or(&Value::Bool(false)).as_bool().unwrap();

  let apps = scoop.installed_apps().unwrap();
  if apps.len() > 0 {
    if brew_list_mode {
      todo!();
    } else {
      println!("Installed apps:");
      for app in apps {
        let version = scoop.current_version(&app.entry).unwrap();
        let install_info = scoop.install_info(&app.entry, &version);

        // name, version
        print!("  {} {}", app.name, version);
        // global
        if app.global {
          print!(" *global*");
        }
        // failed
        if install_info.is_err() {
          print!(" *failed*");
        }
        // hold
        let install_info = install_info.unwrap();
        if install_info.get("hold").is_some() {
          print!(" *hold*");
        }
        // bucket
        let bucket_info = install_info.get("bucket");
        if bucket_info.is_some() {
          print!(" [{}]", bucket_info.unwrap().as_str().unwrap());
        } else if install_info.get("url").is_some() {
          print!(" [{}]", install_info.get("url").unwrap().as_str().unwrap());
        }
        // arch
        // TODO
        print!("\n");
      }
    }
  }
}
