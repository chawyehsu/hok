use clap::ArgMatches;
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use remove_dir_all::remove_dir_all;
use scoop_core::{fs, AppManager, Config};
use sysinfo::ProcessExt;

pub fn cmd_cleanup(matches: &ArgMatches, config: &Config) {
    static RE: Lazy<Regex> = Lazy::new(|| {
        RegexBuilder::new(r".*?apps[\\/]+(?P<app>[a-zA-Z0-9-_.]+)[\\/]+.*")
            .build()
            .unwrap()
    });

    let app_manager = AppManager::new(config);
    let mut sys = scoop_core::sys::SysTool::new();
    let mut running_apps = sys
        .running_apps(config)
        .into_iter()
        .map(|(_, p)| {
            RE.captures(p.exe().to_str().unwrap())
                .unwrap()
                .name("app")
                .unwrap()
                .as_str()
        })
        .collect::<Vec<_>>();
    running_apps.sort();
    running_apps.dedup();

    if matches.is_present("all") {
        let outdated_apps = app_manager.outdated_apps();
        for out in outdated_apps.into_iter() {
            if out.1.len() > 0 {
                let name = out.0;
                if running_apps.contains(&name.as_str()) {
                    eprintln!("Application {} is still running, skip removing.", name);
                    continue;
                }

                print!("Removed {}", name);
                for path in out.1 {
                    remove_dir_all(path.as_path()).expect("failed to remove");
                    print!(" {}", fs::leaf(path.as_path()));
                }
                println!("");
            }
        }
        println!("Everything is shiny now!");
    } else if matches.value_of("app").is_some() {
        let name = matches.value_of("app").unwrap();
        if !app_manager.is_app_installed(name) {
            eprintln!("{} is not installed, skipping cleanup.", name);
        } else {
            let outdated = app_manager.outdated_app(name);
            match outdated {
                None => println!("{} is already clean.", name),
                Some(outdated) => {
                    if outdated.len() > 0 {
                        if running_apps.contains(&name) {
                            eprintln!("Application {} is still running, skip removing.", name);
                        } else {
                            print!("Removed {}", name);
                            for path in outdated {
                                // TODO: Add clean logic
                                remove_dir_all(path.as_path()).expect("failed to remove");
                                print!(" {}", fs::leaf(path.as_path()));
                            }
                            println!("");
                            println!("Everything is shiny now!");
                        }
                    } else {
                        println!("{} is already clean", name);
                    }
                }
            }
        }
    }
}
