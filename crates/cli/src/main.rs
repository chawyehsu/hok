pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(scoop_cli::create_app(args) as i32);
}
