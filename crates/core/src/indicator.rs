use indicatif::{ProgressBar, ProgressStyle};

static BAR_FMT: &'static str = "{bar:68} {percent:>3}% ({eta})";

pub fn pb_download(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(BAR_FMT)
            .progress_chars("## "),
    );
    pb
}
