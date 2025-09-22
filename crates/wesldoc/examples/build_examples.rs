use clap::Parser;
use wesldoc::{Args, Result};

fn main() -> Result<()> {
    Args::parse_from(["wesldoc", "./example_packages/primitives"]).run()?;
    Args::parse_from(["wesldoc", "./example_packages/math_utils"]).run()?;
    Args::parse_from(["wesldoc", "./example_packages/pbr"]).run()?;

    Ok(())
}
