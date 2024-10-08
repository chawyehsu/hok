use clap::ArgMatches;
use crossterm::style::Stylize;
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
                if num >= length {
                    eprintln!("Invalid input.");
                    return Ok(());
                }
                &result[num]
            };

            let path = package.manifest().path();
            println!("{}:", path.display().to_string().green());
            match is_program_available("bat.exe") {
                false => {
                    let content = std::fs::read_to_string(path)?;
                    println!("{}", content.trim());
                }
                true => {
                    let config = session.config();
                    let mut cat_args = vec!["--no-paging"];
                    let cat_style = config.cat_style();
                    if !cat_style.is_empty() {
                        cat_args.push("--style");
                        cat_args.push(cat_style);
                    }
                    cat_args.push("--language");
                    cat_args.push("json");

                    let mut child = Command::new("bat.exe").arg(path).args(cat_args).spawn()?;
                    child.wait()?;
                }
            }
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
