use crossterm::{cursor, style::Stylize, ExecutableCommand};
use libscoop::{operation, Event, Session};
use std::{
    cmp::Ordering,
    io::{stdout, Write},
};

use crate::Result;

pub fn cmd_update(_: &clap::ArgMatches, session: &Session) -> Result<()> {
    let mut stdout = std::io::stdout();
    println!("Updating buckets");
    let _ = stdout.execute(cursor::Hide);
    let rx = session.event_bus().receiver();
    let handle = std::thread::spawn(move || {
        let mut ctx = Context::new();

        while let Ok(event) = rx.recv() {
            match event {
                Event::BucketUpdateStarted(name) => ctx.add(&name),
                Event::BucketUpdateSuccessed(name) => ctx.succeed(&name),
                Event::BucketUpdateFailed(c) => ctx.fail(&c.name),
                Event::BucketUpdateFinished => break,
                _ => {}
            }
        }

        // move cursor to the end
        let mut stdout = std::io::stdout();
        let step = (ctx.data.len() - ctx.cursor) as u16;
        let _ = stdout.execute(cursor::MoveToNextLine(step)).unwrap();
    });
    operation::bucket_update(session)?;
    handle.join().unwrap();
    let _ = stdout.execute(cursor::Show);
    Ok(())
}

enum State {
    Started,
    Failed,
    Successed,
}

struct Context {
    data: Vec<(String, State)>,
    cursor: usize,
}

impl Context {
    pub fn new() -> Context {
        Context {
            data: vec![],
            cursor: 0,
        }
    }

    pub fn add(&mut self, name: &str) {
        for (k, _) in self.data.iter() {
            if k == name {
                return;
            }
        }
        self.data.push((name.to_owned(), State::Started));
        self.draw(name);
    }

    pub fn fail(&mut self, name: &str) {
        for (k, v) in self.data.iter_mut() {
            if k == name {
                *v = State::Failed;
                self.draw(name);
                return;
            }
        }
    }

    pub fn succeed(&mut self, name: &str) {
        for (k, v) in self.data.iter_mut() {
            if k == name {
                *v = State::Successed;
                self.draw(name);
                return;
            }
        }
    }

    pub fn draw(&mut self, name: &str) {
        let idx = self.data.iter().position(|(k, _)| k == name).unwrap();
        let mut stdout = stdout();

        match self.cursor.cmp(&idx) {
            Ordering::Less => {
                let step = (idx - self.cursor) as u16;
                // move cursor down
                stdout.execute(cursor::MoveToNextLine(step)).unwrap();
            }
            Ordering::Greater => {
                let step = (self.cursor - idx) as u16;
                // move cursor up
                stdout.execute(cursor::MoveToPreviousLine(step)).unwrap();
            }
            Ordering::Equal => {}
        }
        self.cursor = idx;

        let state = &self.data.iter().find(|(k, _)| k == name).unwrap().1;

        match state {
            State::Started => {
                println!("{}", name);
                self.cursor += 1;
            }
            State::Successed => print!("{} {}", name, "Ok".green()),
            State::Failed => print!("{} {}", name, "Err".red()),
        }

        std::io::stdout().flush().unwrap();
    }
}
