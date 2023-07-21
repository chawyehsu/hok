pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(hok::create_app(args) as i32);
}
