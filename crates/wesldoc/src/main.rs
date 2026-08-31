use clap::Parser;

fn main() -> anyhow::Result<()> {
    human_panic::setup_panic!();
    init_logger();
    wesldoc::Args::parse().run()?;

    Ok(())
}

#[cfg(debug_assertions)]
fn init_logger() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
}

#[cfg(not(debug_assertions))]
fn init_logger() {
    env_logger::init();
}
