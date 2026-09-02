use clap::Parser;
use wesldoc::Args;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    for package in ["primitives", "math_utils", "pbr"] {
        Args::parse_from([
            "wesldoc",
            &format!("./example_packages/{package}"),
            "--no-deps",
            "--statistics",
        ])
        .run();
    }
}
