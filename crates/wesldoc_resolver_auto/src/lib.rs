#![expect(clippy::type_complexity)]

use anyhow::{Result, bail};
use std::path::Path;
use wesldoc_resolver::{
    ResolverBackend,
    wesl_toml::{WeslToml, WeslTomlPackageManager},
};
use wesldoc_resolver_cargo::CargoResolverBackend;
use wesldoc_resolver_npm::NpmResolverBackend;

pub fn resolver_backend(
    package: &Path,
) -> Result<(
    &'static str,
    Box<dyn FnOnce() -> Result<Box<dyn ResolverBackend>>>,
)> {
    // Load wesl.toml
    let wesl_toml = WeslToml::load(package.join("wesl.toml"))?;

    // Choose the package manager to use
    let package_manager = wesl_toml.as_ref().and_then(|t| t.package.package_manager);

    match package_manager {
        Some(WeslTomlPackageManager::Cargo) => Ok(cargo_backend(package)),
        Some(WeslTomlPackageManager::Npm) => Ok(npm_backend(package)),
        None => {
            match (
                CargoResolverBackend::is_applicable(package)?,
                NpmResolverBackend::is_applicable(package)?,
            ) {
                (true, true) => {
                    wesldoc_report::warn!(
                        "Both Cargo.toml and package.json found. Using Cargo.toml by default. \
                            To use package.json, specify 'package-manager = \"npm\"' in wesl.toml"
                    );
                    Ok(cargo_backend(package))
                }
                (true, false) => Ok(cargo_backend(package)),
                (false, true) => Ok(npm_backend(package)),
                (false, false) => {
                    bail!("the root package must be managed by a package manager (Cargo or npm).");
                }
            }
        }
    }
}

fn cargo_backend(
    package: &Path,
) -> (
    &'static str,
    Box<dyn FnOnce() -> Result<Box<dyn ResolverBackend>>>,
) {
    let package = package.to_path_buf();
    (
        CargoResolverBackend::NAME,
        Box::new(move || Ok(Box::new(CargoResolverBackend::resolve(&package)?))),
    )
}

fn npm_backend(
    package: &Path,
) -> (
    &'static str,
    Box<dyn FnOnce() -> Result<Box<dyn ResolverBackend>>>,
) {
    let package = package.to_path_buf();
    (
        NpmResolverBackend::NAME,
        Box::new(move || Ok(Box::new(NpmResolverBackend::resolve(&package)?))),
    )
}
