use crossterm::{
    cursor,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{collections::HashMap, io::stdout};

static BAR_FMT: &str = " {wide_msg} {total_bytes:>12} [{bar:>20}] {percent:>3}%";

/// Multiple progress bars with own context.
pub struct MultiProgressUI {
    mp: MultiProgress,
    ctx: HashMap<String, HashMap<String, (u64, u64)>>,
    bars: HashMap<String, ProgressBar>,
}

impl MultiProgressUI {
    pub fn new() -> MultiProgressUI {
        MultiProgressUI {
            mp: MultiProgress::new(),
            ctx: HashMap::new(),
            bars: HashMap::new(),
        }
    }

    /// Update progress bar with the given context.
    pub fn update(&mut self, ident: String, url: String, _: String, dltotal: u64, dlnow: u64) {
        if dltotal == 0 {
            return;
        }

        let mut total = 0;
        let mut now = 0;

        self.ctx
            .entry(ident.clone())
            .and_modify(|inner| {
                inner.insert(url.clone(), (dltotal, dlnow));
            })
            .or_insert({
                let mut ctx = HashMap::new();
                ctx.insert(url.clone(), (dltotal, dlnow));
                ctx
            })
            .iter()
            .for_each(|(_, (t, n))| {
                total += t;
                now += n;
            });

        self.bars
            .entry(ident.clone())
            .and_modify(|bar| {
                bar.set_length(total);
                bar.set_position(now);

                if total == now {
                    bar.finish();
                }
            })
            .or_insert_with(|| {
                let bar = self.mp.add(ProgressBar::new(total));
                bar.set_message(ident.clone());
                bar.set_position(0);
                bar.set_style(
                    ProgressStyle::default_bar()
                        .template(BAR_FMT)
                        .unwrap()
                        .progress_chars("#> "),
                );
                bar
            });
    }
}

/// Simple UI for bucket update progress
pub struct BucketUpdateUI {
    pub data: HashMap<String, BucketState>,
    pub cursor: usize,
}

/// Bucket update state
pub enum BucketState {
    Started,
    Failed(String),
    Successed,
}

impl BucketUpdateUI {
    pub fn new() -> BucketUpdateUI {
        BucketUpdateUI {
            data: HashMap::new(),
            cursor: 0,
        }
    }

    /// Add a bucket progress to the UI.
    pub fn add(&mut self, name: &str) {
        self.data.insert(name.to_owned(), BucketState::Started);
        self.draw();
    }

    /// Set the bucket progress to failed.
    pub fn fail(&mut self, name: &str, msg: &str) {
        self.data
            .insert(name.to_owned(), BucketState::Failed(msg.to_owned()));
        self.draw();
    }

    /// Set the bucket progress to successed.
    pub fn succeed(&mut self, name: &str) {
        self.data.insert(name.to_owned(), BucketState::Successed);
        self.draw();
    }

    /// Draw the progress to the stdout.
    pub fn draw(&mut self) {
        let mut stdout = stdout();
        let mut sorted = self.data.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|&(k, _)| k.clone());

        for (name, state) in sorted.iter() {
            let _ = match state {
                BucketState::Started => stdout
                    .execute(Clear(ClearType::CurrentLine))
                    .unwrap()
                    .execute(Print(format!("{}\n", name)))
                    .unwrap(),
                BucketState::Successed => stdout
                    .execute(Clear(ClearType::CurrentLine))
                    .unwrap()
                    .execute(Print(format!("{} {}\n", name, "Ok".green())))
                    .unwrap(),
                BucketState::Failed(_) => stdout
                    .execute(Clear(ClearType::CurrentLine))
                    .unwrap()
                    .execute(Print(format!("{} {}\n", name, "Err".red())))
                    .unwrap(),
            };
        }

        // move cursor back to the first line
        stdout
            .execute(cursor::MoveToPreviousLine(sorted.len() as u16))
            .unwrap();
    }
}
