use crossterm::{
    ExecutableCommand, cursor,
    style::{Print, Stylize},
    terminal::{Clear, ClearType, size},
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{
    collections::HashMap,
    io::{IsTerminal, Write, stderr, stdout},
};

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
                BucketState::Failed(_err) => stdout
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

/// Incremental renderer for archive extraction progress.
///
/// On a terminal the current archive entry is drawn as a single live line on
/// stderr. When stderr is redirected, only the per-archive completion lines are
/// printed, so logs stay readable.
pub struct ExtractUI {
    tty: bool,
    /// Usable entry width for the live line, including the `x ` prefix.
    width: usize,
}

impl ExtractUI {
    pub fn new() -> ExtractUI {
        let width = size()
            .map(|(cols, _)| usize::from(cols).saturating_sub(4))
            .unwrap_or(76);
        ExtractUI {
            tty: stderr().is_terminal(),
            width,
        }
    }

    /// Note that extraction of an archive has started.
    ///
    /// Only meaningful for the live line; kept so a caller can clear any
    /// leftover entry from a previous archive.
    pub fn start(&mut self) {
        self.clear_if_tty();
    }

    /// Report a written archive entry.
    pub fn entry(&mut self, entry: &str) {
        if !self.tty {
            return;
        }

        let shown = fit_entry(entry, self.width);

        let mut stderr = stderr();
        let _ = write!(stderr, "\r\x1b[2K{} {}", "x".yellow(), shown);
        let _ = stderr.flush();
    }

    /// Erase the live line when drawing to a terminal.
    fn clear_if_tty(&mut self) {
        if self.tty {
            let mut stderr = stderr();
            let _ = write!(stderr, "\r\x1b[2K");
            let _ = stderr.flush();
        }
    }

    /// Report that extraction of the current archive has finished.
    pub fn done(
        &mut self,
        package: &str,
        archive: &str,
        format: &str,
        extractor: &str,
        entries: u64,
    ) {
        self.clear_if_tty();

        // `Extracting <pkg>: <archive> (<format>, <extractor>, <n> entries)`
        println!(
            "Extracting {}: {} ({}, {}, {} {})",
            package,
            archive,
            format,
            extractor,
            entries,
            if entries == 1 { "entry" } else { "entries" },
        );
    }
}

impl Default for ExtractUI {
    fn default() -> Self {
        Self::new()
    }
}

/// Shorten an entry path to `...\<basename>` when it does not fit `width`.
fn fit_entry(entry: &str, width: usize) -> String {
    if entry.chars().count() <= width {
        return entry.to_owned();
    }

    let basename = entry
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(entry);
    let indented = format!(r"...\{basename}");

    if indented.chars().count() <= width {
        return indented;
    }

    // Keep the tail of the basename, which carries the most signal.
    let keep = width.saturating_sub(4);
    let chars = basename.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(keep);
    format!(r"...\{}", chars[start..].iter().collect::<String>())
}

/// Prompt user to continue or not.
pub fn prompt_yes_no() -> bool {
    loop {
        print!("\nDo you want to continue? [y/N]: ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let c = input.trim_end();
        if c.chars().count() == 1 {
            let ch: char = c.chars().next().unwrap();
            if ['y', 'Y', 'n', 'N'].contains(&ch) {
                return ch == 'y' || ch == 'Y';
            }
        }
    }
}
