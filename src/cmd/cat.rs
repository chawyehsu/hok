use clap::ArgMatches;
use libscoop::{operation, QueryOption, Session};
use std::{io::Write, path::Path, process::Command};

use crate::Result;

pub fn cmd_cat(matches: &ArgMatches, session: &Session) -> Result<()> {
    if let Some(query) = matches.get_one::<String>("package") {
        let queries = vec![query.as_str()];
        let options = vec![QueryOption::Explicit];
        let mut result = operation::package_query(session, queries, options, false)?;

        if result.is_empty() {
            eprintln!("Could not find package named '{}'.", query)
        } else {
            let length = result.len();
            let package = if length == 1 {
                &result[0]
            } else {
                result.sort_by_key(|p| p.ident());

                println!("Found multiple packages named '{}':\n", query);
                for (idx, pkg) in result.iter().enumerate() {
                    println!(
                        "  {}. {}/{} ({})",
                        idx,
                        pkg.bucket(),
                        pkg.name(),
                        pkg.homepage()
                    );
                }
                print!("\nPlease select one, enter the number to continue: ");
                std::io::stdout().flush().unwrap();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                let parsed = input.trim().parse::<usize>();
                if parsed.is_err() {
                    eprintln!("Invalid input.");
                    return Ok(());
                }

                let num = parsed.unwrap();
                if num >= result.len() {
                    eprintln!("Invalid input.");
                    return Ok(());
                }
                &result[num]
            };

            let cat = match is_program_available("bat.exe") {
                true => "bat.exe",
                false => "type",
            };
            let config = session.config();
            let cat_args = match cat == "bat.exe" {
                false => vec![],
                true => {
                    let cat_style = config.cat_style();
                    vec!["--no-paging", "--style", cat_style, "--language", "json"]
                }
            };

            if length > 1 {
                println!();
            }

            let mut child = Command::new("cmd")
                .arg("/C")
                .arg(cat)
                .arg(package.manfest_path())
                .args(cat_args)
                .spawn()?;
            child.wait()?;
        }
    }
    Ok(())
}

/// Check if a given executable is available on the system
fn is_program_available(exe: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for p in path.split(';') {
            let path = Path::new(p).join(exe);
            if std::fs::metadata(path).is_ok() {
                return true;
            }
        }
    }
    false
}
