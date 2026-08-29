use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Package, PackageId};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use wesldoc_ast::Version;

pub struct CargoMetadata {
    packages: HashMap<PackageId, CargoPackage>,
    root_id: PackageId,
}

impl CargoMetadata {
    pub fn resolve(base_path: impl Into<PathBuf>) -> Result<Self> {
        let base_path = base_path.into();
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(base_path.join("Cargo.toml"))
            .exec()?;

        let mut packages = metadata
            .packages
            .iter()
            .map(|package| {
                (
                    package.id.clone(),
                    CargoPackage {
                        package: package.clone(),
                        deps: HashMap::default(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let resolve = metadata.resolve.as_ref().context("missing resolve")?;
        for node in &resolve.nodes {
            let package = packages.get_mut(&node.id).context("invalid node")?;
            for dep in &node.deps {
                if dep
                    .dep_kinds
                    .iter()
                    .any(|dep_kind_info| dep_kind_info.kind == DependencyKind::Normal)
                {
                    package.deps.insert(dep.name.clone(), dep.pkg.clone());
                }
            }
        }

        let root_id = metadata
            .root_package()
            .context("no root package")?
            .id
            .clone();

        Ok(Self { packages, root_id })
    }

    pub fn package(&self, package_id: &PackageId) -> Option<&CargoPackage> {
        self.packages.get(package_id)
    }

    pub fn iter_packages(&self, max_depth: usize) -> impl Iterator<Item = &CargoPackage> {
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
    }
}

struct IterPackages<'a> {
    metadata: &'a CargoMetadata,
    max_depth: usize,

    stack: Vec<IterPackagesCurrent<'a>>,
    visited: HashSet<&'a PackageId>,
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
    deps: std::collections::hash_map::Values<'a, String, PackageId>,
}

#[derive(Debug, Clone)]
pub struct CargoPackage {
    package: Package,
    deps: HashMap<String, PackageId>,
}

impl CargoPackage {
    pub fn id(&self) -> PackageId {
        self.package.id.clone()
    }

    pub fn name(&self) -> String {
        self.package.name.to_string()
    }

    pub fn version(&self) -> Version {
        self.package.version.clone()
    }

    pub fn crate_path(&self) -> PathBuf {
        self.package
            .manifest_path
            .as_std_path()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    pub fn dep(&self, name: &str) -> Option<&PackageId> {
        self.deps.get(name)
    }
}
