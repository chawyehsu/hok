use crossterm::{cursor, ExecutableCommand};
use libscoop::{operation, Event, Session};

use crate::{cui, Result};

pub fn cmd_update(_: &clap::ArgMatches, session: &Session) -> Result<()> {
    let rx = session.event_bus().receiver();

    let handle = std::thread::spawn(move || {
        let mut progress = cui::BucketUpdateUI::new();

        while let Ok(event) = rx.recv() {
            match event {
                Event::BucketUpdateProgress(ctx) => {
                    if ctx.state().started() {
                        progress.add(ctx.name());
                    } else if ctx.state().succeeded() {
                        progress.succeed(ctx.name());
                    } else {
                        let err_msg = ctx.state().failed().unwrap();
                        progress.fail(ctx.name(), err_msg);
                    }
                }
                Event::BucketUpdateDone => break,
                _ => {}
            }
        }

        // move cursor to the end
        let mut stdout = std::io::stdout();
        let step = (progress.data.len() - progress.cursor) as u16;
        let _ = stdout.execute(cursor::MoveToNextLine(step)).unwrap();
    });

    println!("Updating buckets");

    let mut stdout = std::io::stdout();
    let _ = stdout.execute(cursor::Hide);

    operation::bucket_update(session)?;

    handle.join().unwrap();

    let _ = stdout.execute(cursor::Show);

    Ok(())
}
