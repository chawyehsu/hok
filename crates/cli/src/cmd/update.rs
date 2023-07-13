use console::{style, Term};
use scoop_core::{bucket::BucketUpdateState, Session};
use std::{collections::HashMap, io::Write};

use crate::Result;

pub fn cmd_update(_: &clap::ArgMatches, session: &mut Session) -> Result<()> {
    let mut term = Term::stdout();
    let mut collected_ctx = HashMap::new();

    session
        .bucket_update(move |ret| {
            let last_len = collected_ctx.len();
            collected_ctx.insert(ret.name.clone(), ret);

            let mut items = collected_ctx.iter().map(|(_, ctx)| ctx).collect::<Vec<_>>();
            items.sort_by_key(|&ctx| ctx.name.clone());
            let mut w = vec![];
            for item in items {
                match &item.state {
                    BucketUpdateState::Started => {
                        w.push(format!("Updating '{}' bucket...\n", item.name))
                    }
                    BucketUpdateState::Successed => w.push(format!(
                        "Updating '{}' bucket... {}\n",
                        item.name,
                        style("OK").green()
                    )),
                    BucketUpdateState::Failed(e) => w.push(format!(
                        "Updating '{}' bucket... {}{}{}\n",
                        item.name,
                        style("ERR(").red(),
                        e.replace("\r\n", " "),
                        style(")").red()
                    )),
                }
            }
            let _ = term.clear_last_lines(last_len);
            let _ = term.write(w.join("").as_bytes());
        })
        .map_err(|e| e.into())
}
