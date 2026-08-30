use crate::{
    cargo::{CargoMetadata, CargoPackage},
    wesl_toml::{WeslToml, WeslTomlDependency},
};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};
use wesldoc_ast::Version;

#[derive(Debug, Clone)]
pub struct Package {
    pub package_name: String,
    pub version: Version,
    pub wesl_toml: WeslToml,
    pub has_wesl_toml_file: bool,
    pub root: PathBuf,

    pub id: PackageId,
}

impl Package {
    pub fn from_cargo_package(cargo_package: &CargoPackage) -> Result<Self> {
        let (wesl_toml, has_wesl_toml_file) =
            load_wesl_toml(cargo_package.crate_path().join("wesl.toml"))?;

        let package_name = cargo_package.name();
        let version = cargo_package.version();
        let root = cargo_package.crate_path().join(&wesl_toml.package.root);

        Ok(Self {
            package_name,
            version,
            wesl_toml,
            has_wesl_toml_file,
            root,

            id: PackageId::Cargo(cargo_package.id()),
        })
    }

    pub fn from_path(path: &Path, name: String) -> Result<Self> {
        let path = path.canonicalize()?;
        let (wesl_toml, has_wesl_toml_file) = load_wesl_toml(path.join("wesl.toml"))?;

        Ok(Self {
            package_name: name.clone(),
            root: path.join(&wesl_toml.package.root),
            wesl_toml,
            has_wesl_toml_file,
            version: Version::new(0, 0, 0), // TODO: path dependencies don't have versions

            id: PackageId::Path(path, name),
        })
    }

    pub fn new_dependency(
        this_cargo_package: &CargoPackage,

        dependency_key: impl Into<String>,
        dependency: Option<&WeslTomlDependency>,
        cargo_metadata: &CargoMetadata,
    ) -> Result<Self> {
        let dependency_key = dependency_key.into();
        let dep_name = dependency
            .and_then(|d| d.package.as_ref())
            .unwrap_or(&dependency_key);

        // Handle path dependencies
        if let Some(dep_path) = dependency.and_then(|d| d.path.as_ref()) {
            return Self::from_path(
                &this_cargo_package.crate_path().join(dep_path),
                dep_name.clone(),
            );
        }

        let dep_pkg_id = this_cargo_package
            .dep(dep_name)
            .with_context(|| format!("dependency '{dep_name}' not found in Cargo.toml"))?;
        let dep_pkg = cargo_metadata
            .package(dep_pkg_id)
            .context("invalid dependency")?;
        Package::from_cargo_package(dep_pkg)
    }

    // Only count as a wesl package if it has a wesl.toml file or at least one .wesl file
    pub fn is_wesl_package(&self) -> Result<bool> {
        if self.has_wesl_toml_file {
            return Ok(true);
        }

        if !self.root.is_dir() {
            return Ok(false);
        }
        let mut dirs = vec![self.root.clone()];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() && path.extension().is_some_and(|ext| ext == "wesl") {
                    return Ok(true);
                } else if path.is_dir() {
                    dirs.push(path);
                }
            }
        }

        Ok(false)
    }
}

fn load_wesl_toml(path: impl AsRef<Path>) -> Result<(WeslToml, bool)> {
    let path = path.as_ref();
    let (wesl_toml, has_wesl_toml_file) = if path.is_file() {
        (toml::from_slice::<WeslToml>(&fs::read(path)?)?, true)
    } else {
        (WeslToml::default(), false)
    };
    wesl_toml.validate()?;
    Ok((wesl_toml, has_wesl_toml_file))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageId {
    Cargo(cargo_metadata::PackageId),
    Path(PathBuf, String), // Path and package name
}
