mod preprocess;

use crate::{
    cargo::CargoMetadata,
    package::{Package, PackageId},
};
use anyhow::Result;
use std::{
    collections::{HashMap, hash_map::Entry},
    fs,
    path::Path,
    sync::Arc,
};
use wesldoc_compiler::WeslModule;

pub struct BuildCache {
    packages: HashMap<PackageId, Arc<WeslModule>>,
    cargo_metadata: Arc<CargoMetadata>,
}

impl BuildCache {
    pub fn new(cargo_metadata: Arc<CargoMetadata>) -> Self {
        Self {
            packages: HashMap::new(),
            cargo_metadata,
        }
    }

    pub fn get_or_build(&mut self, package_id: PackageId) -> Result<Option<Arc<WeslModule>>> {
        match self.packages.entry(package_id) {
            Entry::Occupied(pkg) => Ok(Some(Arc::clone(pkg.get()))),
            Entry::Vacant(entry) => {
                let package = match entry.key() {
                    PackageId::Cargo(package_id) => {
                        let cargo_package = match self.cargo_metadata.package(package_id) {
                            Some(pkg) => pkg,
                            None => return Ok(None),
                        };
                        Package::from_cargo_package(cargo_package)?
                    }
                    PackageId::Path(_path_buf) => todo!(),
                };
                let wesl_module = Arc::new(build_package(package)?);
                entry.insert(Arc::clone(&wesl_module));
                Ok(Some(wesl_module))
            }
        }
    }

    pub fn cargo_metadata(&self) -> &CargoMetadata {
        &self.cargo_metadata
    }
}

fn build_package(package: Package) -> Result<WeslModule> {
    Ok(WeslModule {
        name: package.package_name,
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
