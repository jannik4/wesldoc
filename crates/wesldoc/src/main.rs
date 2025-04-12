use clap::Parser;

fn main() -> wesldoc::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    wesldoc::Args::parse().run()?;

    Ok(())
}
