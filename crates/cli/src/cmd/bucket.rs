use clap::ArgMatches;
use console::{style, Term};
use scoop_core::Session;

use crate::Result;

pub fn cmd_bucket(matches: &ArgMatches, session: &Session) -> Result<()> {
    match matches.subcommand() {
        ("add", Some(args)) => {
            let name = args.value_of("name").unwrap_or_default();
            let repo = args.value_of("repo").unwrap_or_default();
            match session.bucket_add(name, repo) {
                Err(e) => Err(e.into()),
                Ok(_) => Ok(()),
            }
        }
        ("known", Some(_)) => {
            let term = Term::stdout();
            for (name, repo) in session.bucket_known() {
                let _ = term.write_line(format!("{:<8} {}", style(name).green(), repo).as_str());
            }
            return Ok(());
        }
        ("list", Some(_)) => match session.bucket_list() {
            Err(e) => Err(e.into()),
            Ok(buckets) => {
                let term = Term::stdout();
                for bucket in buckets {
                    let _ = term.write_line(
                        format!("{} {}", style(bucket.name()).green(), bucket.repository())
                            .as_str(),
                    );
                }
                return Ok(());
            }
        },
        ("remove", Some(args)) => {
            let name = args.value_of("name").unwrap_or_default();
            match session.bucket_remove(name) {
                Err(e) => Err(e.into()),
                Ok(_) => Ok(()),
            }
        }
        _ => unreachable!(),
    }
}
