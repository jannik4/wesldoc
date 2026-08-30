mod preprocess;

use crate::{
    cargo::CargoMetadata,
    package::{Package, PackageId},
    wesl_toml::DependenciesAuto,
};
use anyhow::{Context, Result};
use either::Either;
use std::{
    collections::{HashMap, hash_map::Entry},
    fs,
    path::Path,
    sync::Arc,
};
use wesldoc_compiler::WeslModule;

pub struct BuildCache {
    packages: HashMap<PackageId, PackageBuild>,
    cargo_metadata: Arc<CargoMetadata>,
}

impl BuildCache {
    pub fn new(cargo_metadata: Arc<CargoMetadata>) -> Self {
        Self {
            packages: HashMap::new(),
            cargo_metadata,
        }
    }

    pub fn get_or_build(&mut self, package_id: PackageId) -> Result<Option<&mut PackageBuild>> {
        match self.packages.entry(package_id) {
            Entry::Occupied(pkg) => Ok(Some(pkg.into_mut())),
            Entry::Vacant(entry) => {
                let package = PackageBuild::new(entry.key(), &self.cargo_metadata)?;
                Ok(Some(entry.insert(package)))
            }
        }
    }

    pub fn get(&self, package_id: &PackageId) -> Option<&PackageBuild> {
        self.packages.get(package_id)
    }
}

pub struct PackageBuild {
    pub package: Arc<Package>,
    pub build: Arc<WeslModule>,
    pub dependencies: Dependencies,
}

impl PackageBuild {
    fn new(package_id: &PackageId, cargo_metadata: &CargoMetadata) -> Result<Self> {
        let (package, this_package_meta) = match package_id {
            PackageId::Cargo(package_id) => {
                let cargo_package = cargo_metadata
                    .package(package_id)
                    .context("expected cargo package")?;
                (
                    Package::from_cargo_package(cargo_package)?,
                    Either::Left(cargo_package),
                )
            }
            PackageId::Path(path, name) => (
                Package::from_path(path, name.clone())?,
                Either::Right(&**path),
            ),
        };

        let build = Arc::new(build_package(&package)?);
        let dependencies = match package.wesl_toml.package.dependencies {
            Some(DependenciesAuto::Auto) => Dependencies::Auto {
                dependencies: HashMap::new(),
            },
            None => Dependencies::Explicit {
                dependencies: package
                    .wesl_toml
                    .dependencies
                    .iter()
                    .map(|(dep_key, dep)| {
                        Ok((
                            dep_key.clone(),
                            Arc::new(Package::new_dependency(
                                this_package_meta,
                                dep_key,
                                Some(dep),
                                cargo_metadata,
                            )?),
                        ))
                    })
                    .collect::<Result<_>>()?,
            },
        };

        Ok(Self {
            package: Arc::new(package),
            build,
            dependencies,
        })
    }
}

// local_name -> package
#[derive(Debug, Clone)]
pub enum Dependencies {
    Explicit {
        dependencies: HashMap<String, Arc<Package>>,
    },
    Auto {
        dependencies: HashMap<String, Arc<Package>>,
    },
}

impl Dependencies {
    pub fn into_iter(self) -> impl Iterator<Item = Arc<Package>> {
        match self {
            Dependencies::Explicit { dependencies } => dependencies.into_values(),
            Dependencies::Auto { dependencies } => dependencies.into_values(),
        }
    }
}

fn build_package(package: &Package) -> Result<WeslModule> {
    Ok(WeslModule {
        name: package.package_name.clone(),
        code: None,
        submodules: build_submodules(&package.root)?,
    })
}

fn build_submodules(dir: &Path) -> Result<Vec<WeslModule>> {
    let mut submodules = HashMap::new();

    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        let name = name_from_path(&path)?;

        if path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext == "wesl" || ext == "wgsl")
        {
            let sub = submodules
                .entry(name)
                .or_insert_with_key(|name| WeslModule {
                    name: name.clone(),
                    code: None,
                    submodules: Vec::new(),
                });

            let source = fs::read_to_string(&path)?;
            let syntax = self::preprocess::preprocess(wgsl_parse::parse_str(&source)?)?;

            sub.code = Some((Arc::new(syntax), source))
        } else if path.is_dir() {
            let sub = submodules
                .entry(name)
                .or_insert_with_key(|name| WeslModule {
                    name: name.clone(),
                    code: None,
                    submodules: Vec::new(),
                });
            sub.submodules = build_submodules(&path)?;
        }
    }

    Ok(submodules
        .into_values()
        .filter(|module| module.code.is_some() || !module.submodules.is_empty())
        .collect())
}

fn name_from_path(path: &Path) -> Result<String> {
    let path = path.canonicalize()?;
    Ok(path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .replace('-', "_"))
}
