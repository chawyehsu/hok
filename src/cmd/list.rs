use clap::ArgMatches;

use crate::Scoop;

pub fn cmd_list(_matches: &ArgMatches, scoop: &mut Scoop) {
  let brew_list_mode = false;

  let apps = scoop.apps_manager.installed_apps();
  if apps.len() > 0 {
    if brew_list_mode {
      todo!();
    } else {
      println!("Installed apps:");
      for app in apps {
        let version = app.current_version();
        let install_info = app.current_install_info();

        // name, version
        print!("  {} {}", app.name, version);
        // global
        // if app.global {
        //   print!(" *global*");
        // }
        // failed
        if install_info.is_err() {
          print!(" *failed*");
        }
        // hold
        let install_info = install_info.unwrap();
        if install_info.hold.is_some() {
          print!(" *hold*");
        }
        // bucket
        if install_info.bucket.is_some() {
          print!(" [{}]", install_info.bucket.unwrap());
        } else if install_info.url.is_some() {
          print!(" [{}]", install_info.url.unwrap());
        }
        // arch
        // print!(" [{}]", install_info.architecture);
        // TODO
        print!("\n");
      }
    }
  }
}
