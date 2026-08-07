use dirsnap::Assert;
use wesldoc::{Args, Parser, Result};

#[test]
fn example_packages() -> Result<()> {
    snapshot_test(
        "example_packages",
        &[
            "../example_packages/primitives",
            "../example_packages/math_utils",
            "../example_packages/pbr",
        ],
    )
}

fn snapshot_test(name: &str, packages: &[&str]) -> Result<()> {
    let tmp_dir = tempfile::tempdir()?;

    for package in packages {
        build_package(package, &tmp_dir)?;
    }

    Assert::new()
        .with_action_env_var("SNAPSHOTS")
        .ignore("-/static")
        .eq(format!("tests/snapshots/{name}"), tmp_dir.path());

    Ok(())
}

fn build_package(package_path: &str, tmp_dir: &tempfile::TempDir) -> Result<()> {
    let output_path = tmp_dir.path();

    Args::parse_from([
        "wesldoc",
        package_path,
        "--output",
        output_path.to_str().unwrap(),
    ])
    .run()?;

    Ok(())
}
