use clap::Parser;
use wesldoc::Args;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    Args::parse_from(["wesldoc", "./example_packages/primitives"]).run()?;
    Args::parse_from(["wesldoc", "./example_packages/math_utils"]).run()?;
    Args::parse_from(["wesldoc", "./example_packages/pbr"]).run()?;

    Ok(())
}
