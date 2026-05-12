use clap::Args as ClapArgs;
use libscoop::operation;
use libscoop::Session;

#[derive(Debug, ClapArgs)]
pub struct Args {
    #[arg(short = 'l', long)]
    pub local: bool,

    #[arg(long)]
    pub json: bool,
}

pub fn execute(args: Args, session: &Session) -> anyhow::Result<()> {
    let report = operation::status(session, args.local)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    if report.scoop_outdated {
        println!("Scoop out of date. Run 'scoop update' to get the latest changes.");
    }
    if report.buckets_outdated {
        println!("Scoop bucket(s) out of date. Run 'scoop update' to get the latest changes.");
    }
    if !report.scoop_outdated && !report.buckets_outdated && report.entries.is_empty() {
        println!("Everything is ok!");
        return Ok(());
    }

    for entry in report.entries {
        println!(
            "{: <20} {: <15} {: <15} {: <20} {}",
            entry.name,
            entry.installed_version.unwrap_or_default(),
            entry.latest_version.unwrap_or_default(),
            entry.missing_dependencies.join(" | "),
            entry.flags.iter().map(|f| f.as_str()).collect::<Vec<_>>().join(", ")
        );
    }

    Ok(())
}
