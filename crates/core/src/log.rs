use env_logger::Env;

pub fn create_logger() {
    let env = Env::default()
        .filter_or("SCOOP_LOG_LEVEL", "trace")
        .write_style("never");

    env_logger::init_from_env(env);
}
