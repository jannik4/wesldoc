use crate::{
    ResolverBackend, ResolverBackendPackage,
    wesl_toml::{WeslToml, WeslTomlDependency},
};
use anyhow::{Context, Result, bail};
use either::Either;
use std::{
    fs,
    path::{Path, PathBuf},
};
use wesldoc_ast::Version;

// TODO: better name?
#[derive(Debug, Clone)]
pub struct Package {
    pub package_name: String,
    pub version: Version,
    pub wesl_toml: WeslToml,
    pub has_wesl_toml_file: bool,
    pub root: PathBuf,

    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,

    pub id: PackageId,
}

impl Package {
    pub fn from_path(path: &Path, name: String) -> Result<Self> {
        let path = path.canonicalize()?;
        let (wesl_toml, has_wesl_toml_file) = load_wesl_toml(path.join("wesl.toml"))?;

        Ok(Self {
            package_name: name.clone(),
            root: path.join(&wesl_toml.package.root),
            wesl_toml,
            has_wesl_toml_file,
            version: Version::new(0, 0, 0), // TODO: path dependencies don't have versions

            homepage: None,
            repository: None,
            license: None,

            id: PackageId::Path {
                canonical_path: path,
            },
        })
    }

    pub fn new_dependency(
        // either a backend package or a path to the package
        this_package: Either<&dyn ResolverBackendPackage, &Path>,

        dependency_key: impl Into<String>,
        dependency: Option<&WeslTomlDependency>,
        backend: &dyn ResolverBackend,
    ) -> Result<Self> {
        let dependency_key = dependency_key.into();
        let dep_name = dependency
            .and_then(|d| d.package.as_ref())
            .unwrap_or(&dependency_key);

        // Handle path dependencies
        if let Some(dep_path) = dependency.and_then(|d| d.path.as_ref()) {
            let base_path = match this_package {
                Either::Left(backend_package) => &backend_package.path(),
                Either::Right(path) => path,
            };

            return Self::from_path(&base_path.join(dep_path), dep_name.clone());
        }

        let this_backend_package = match this_package {
            Either::Left(backend_package) => backend_package,
            Either::Right(_) => bail!("dependency '{dep_name}' not found"),
        };

        let dep_pkg_id = this_backend_package
            .get_dependency(dep_name)
            .with_context(|| format!("dependency '{dep_name}' not found in manifest"))?;
        let dep_pkg = backend.package(dep_pkg_id).context("invalid dependency")?;
        dep_pkg.to_package()
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
    match WeslToml::load(path)? {
        Some(wesl_toml) => Ok((wesl_toml, true)),
        None => Ok((WeslToml::default(), false)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageId {
    Backend(PackageIdBackend),
    Path { canonical_path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageIdBackend(pub String);
