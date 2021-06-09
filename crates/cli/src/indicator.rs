use indicatif::{ProgressBar, ProgressStyle};

static BAR_FMT: &'static str =
    "{spinner:.green} {msg} ({total_bytes}) [{wide_bar:.cyan/blue}] {percent}% ({eta})";

pub fn pb_download(msg: &str, len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(BAR_FMT)
            .progress_chars("=> "),
    );
    pb.set_message(format!("{}", msg));
    pb
}
