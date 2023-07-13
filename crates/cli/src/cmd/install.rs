use clap::ArgMatches;
use indicatif::{ProgressBar, ProgressStyle};
use scoop_core::{
    package::{DownloadProgressState, InstallOption},
    Session,
};
use std::collections::HashSet;

use crate::Result;

pub fn cmd_install(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(queries) = matches.values_of("package") {
        let query = queries.collect::<Vec<&str>>().join(" ");
        let mut options = HashSet::new();
        if matches.is_present("download-only") {
            options.insert(InstallOption::DownloadOnly);
        }
        if matches.is_present("ignore-broken") {
            options.insert(InstallOption::IgnoreFailure);
        }
        if matches.is_present("no-cache") {
            options.insert(InstallOption::NoCache);
        }
        if matches.is_present("no-hash-check") {
            options.insert(InstallOption::NoHashCheck);
        }

        let mut pb = None;
        let mut throttle = 0;

        session.package_install(&query, options, move |ret| match ret.state {
            DownloadProgressState::Prepared => {
                if pb.is_none() || ret.index == 1 {
                    let bar = ProgressBar::new(ret.total);
                    bar.set_style(
                        ProgressStyle::with_template(
                            "{msg}\n[{bar:40.cyan/blue}] {bytes}/{total_bytes} ({elapsed_precise}, {bytes_per_sec})",
                        )
                        .unwrap()
                        .progress_chars("=>-"),
                    );
                    let msg = format!("Downloading {} {}/{}", ret.name, ret.index, ret.file_count);
                    bar.set_message(msg);
                    pb = Some(bar);
                } else {
                    let bar = pb.as_mut().unwrap();
                    bar.set_length(ret.total);
                    let msg = format!("Downloading {} {}/{}", ret.name, ret.index, ret.file_count);
                    bar.set_message(msg);
                }
            }
            DownloadProgressState::Downloading => {
                if throttle == 10 {
                    let bar = pb.as_mut().unwrap();
                    bar.set_position(ret.position);
                    throttle = 0;
                } else {
                    throttle += 1;
                }
            }
            DownloadProgressState::Finished => {
                let bar = pb.as_mut().unwrap();
                bar.finish();
                if ret.index != ret.file_count {
                    bar.reset();
                }
            }
        })?;
    }
    Ok(())
}
