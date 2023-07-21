use clap::ArgMatches;
use libscoop::{operation, Session};

use crate::Result;

pub fn cmd_cache(matches: &ArgMatches, session: &Session) -> Result<()> {
    match matches.subcommand() {
        Some(("list", args)) => {
            let query = args
                .get_one::<String>("query")
                .map(|s| s.as_ref())
                .unwrap_or("*");
            let files = operation::cache_list(session, query)?;
            let mut total_size: u64 = 0;
            let total_count = files.len();

            for f in files.into_iter() {
                let size = f.path().metadata()?.len();
                total_size += size;

                println!(
                    "{:>8} {} ({}) {:>}",
                    size_bytes(size, true),
                    f.package_name(),
                    f.version(),
                    f.file_name()
                );
            }

            println!(
                "{:>8} {} files, {}",
                "Total:",
                total_count,
                filesize(total_size, true)
            );

            Ok(())
        }
        Some(("remove", args)) => {
            if args.get_flag("all") {
                match operation::cache_remove(session, "*") {
                    Ok(_) => println!("All download caches were removed."),
                    Err(e) => return Err(e.into()),
                }
            }
            if let Some(query) = args.get_one::<String>("query").map(|s| s.as_ref()) {
                match operation::cache_remove(session, query) {
                    Ok(_) => {
                        if query == "*" {
                            println!("All download caches were removed.");
                        } else {
                            println!("All caches matching '{}' were removed.", query);
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            Ok(())
        }
        _ => unreachable!(),
    }
}

/// Get file size of this `CacheFile` in bytes
fn size_bytes(size: u64, unit: bool) -> String {
    filesize(size, unit)
}

/// Convert bytes to KB/MB/GB representation.
fn filesize(length: u64, with_unit: bool) -> String {
    let gb: f64 = 2.0_f64.powf(30_f64);
    let mb: f64 = 2.0_f64.powf(20_f64);
    let kb: f64 = 2.0_f64.powf(10_f64);

    let flength = length as f64;

    if flength > gb {
        let j = (flength / gb).round();

        if with_unit {
            format!("{} GB", j)
        } else {
            j.to_string()
        }
    } else if flength > mb {
        let j = (flength / mb).round();

        if with_unit {
            format!("{} MB", j)
        } else {
            j.to_string()
        }
    } else if flength > kb {
        let j = (flength / kb).round();

        if with_unit {
            format!("{} KB", j)
        } else {
            j.to_string()
        }
    } else {
        if with_unit {
            format!("{} B", flength)
        } else {
            flength.to_string()
        }
    }
}
