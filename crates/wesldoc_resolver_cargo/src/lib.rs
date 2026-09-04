use anyhow::{Context, Result};
use cargo_metadata::DependencyKind;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use wesldoc_resolver::{
    ResolverBackend, ResolverBackendPackage,
    package::{Package, PackageId, PackageIdBackend},
    wesl_toml::WeslToml,
};

pub struct CargoResolverBackend {
    packages: HashMap<PackageIdBackend, CargoPackage>,
    root_id: PackageIdBackend,
}

impl CargoResolverBackend {
    pub const NAME: &'static str = "cargo";

    pub fn is_applicable(base_path: &Path) -> Result<bool> {
        Ok(fs::exists(base_path.join("Cargo.toml"))?)
    }

    pub fn resolve(base_path: &Path) -> Result<Self> {
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(base_path.join("Cargo.toml"))
            .exec()?;

        let root_id = PackageIdBackend(
            metadata
                .root_package()
                .context("no root package")?
                .id
                .repr
                .clone(),
        );

        let mut packages = metadata
            .packages
            .iter()
            .map(|package| {
                (
                    PackageIdBackend(package.id.repr.clone()),
                    CargoPackage {
                        package: package.clone(),
                        deps: HashMap::default(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let resolve = metadata.resolve.context("missing resolve")?;
        for node in resolve.nodes {
            let package = packages
                .get_mut(&PackageIdBackend(node.id.repr))
                .context("invalid node")?;
            for dep in &node.deps {
                if dep
                    .dep_kinds
                    .iter()
                    .any(|dep_kind_info| dep_kind_info.kind == DependencyKind::Normal)
                {
                    package
                        .deps
                        .insert(dep.name.clone(), PackageIdBackend(dep.pkg.repr.clone()));
                }
            }
        }

        Ok(Self { packages, root_id })
    }
}

impl ResolverBackend for CargoResolverBackend {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn package(&self, package_id: &PackageIdBackend) -> Option<&dyn ResolverBackendPackage> {
        self.packages.get(package_id).map(|pkg| pkg as _)
    }

    fn iter_packages(
        &self,
        max_depth: usize,
    ) -> Box<dyn Iterator<Item = &dyn wesldoc_resolver::ResolverBackendPackage> + '_> {
        Box::new(
            self.packages
                .get(&self.root_id)
                .into_iter()
                .flat_map(move |package| IterPackages {
                    metadata: self,
                    max_depth,
                    stack: vec![IterPackagesCurrent {
                        package,
                        deps: package.deps.values(),
                    }],
                    visited: HashSet::new(),
                })
                .map(|pkg| pkg as _),
        )
    }
}

struct IterPackages<'a> {
    metadata: &'a CargoResolverBackend,
    max_depth: usize,

    stack: Vec<IterPackagesCurrent<'a>>,
    visited: HashSet<&'a PackageIdBackend>,
}

impl<'a> Iterator for IterPackages<'a> {
    type Item = &'a CargoPackage;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.stack.len() <= self.max_depth
                && let Some(dep_id) = self.stack.last_mut()?.deps.next()
            {
                if !self.visited.insert(dep_id) {
                    continue;
                }
                if let Some(dep_package) = self.metadata.packages.get(dep_id) {
                    self.stack.push(IterPackagesCurrent {
                        package: dep_package,
                        deps: dep_package.deps.values(),
                    });
                    continue;
                }
            }

            let current = self.stack.pop()?;
            return Some(current.package);
        }
    }
}

struct IterPackagesCurrent<'a> {
    package: &'a CargoPackage,
    deps: std::collections::hash_map::Values<'a, String, PackageIdBackend>,
}

#[derive(Debug, Clone)]
pub struct CargoPackage {
    package: cargo_metadata::Package,
    deps: HashMap<String, PackageIdBackend>,
}

impl ResolverBackendPackage for CargoPackage {
    fn path(&self) -> PathBuf {
        self.package
            .manifest_path
            .as_std_path()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn to_package(&self) -> Result<Package> {
        let (wesl_toml, has_wesl_toml_file) = match WeslToml::load(self.path().join("wesl.toml"))? {
            Some(wesl_toml) => (wesl_toml, true),
            None => (WeslToml::default(), false),
        };
        let root = self.path().join(&wesl_toml.package.root);

        Ok(Package {
            package_name: self.package.name.to_string(),
            version: self.package.version.clone(),
            wesl_toml,
            has_wesl_toml_file,
            root,

            homepage: self.package.homepage.clone(),
            repository: self.package.repository.clone(),
            license: self.package.license.clone(),

            id: PackageId::Backend(PackageIdBackend(self.package.id.repr.clone())),
        })
    }

    fn get_dependency(&self, name: &str) -> Option<&PackageIdBackend> {
        self.deps.get(name)
    }
}
