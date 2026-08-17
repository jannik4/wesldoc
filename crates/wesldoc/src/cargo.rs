use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Package, PackageId};
use std::{collections::HashMap, path::PathBuf};
use wesldoc_ast::Version;

pub struct CargoMetadata {
    packages: HashMap<PackageId, CargoPackage>,
}

impl CargoMetadata {
    pub fn resolve(base_path: impl Into<PathBuf>) -> Result<(Self, CargoPackage)> {
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

        let root_package = &metadata.root_package().context("no root package")?.id;
        let root_package = packages
            .get(root_package)
            .context("invalid root package")?
            .clone();

        Ok((Self { packages }, root_package))
    }

    pub fn package(&self, package_id: &PackageId) -> Option<&CargoPackage> {
        self.packages.get(package_id)
    }
}

#[derive(Debug, Clone)]
pub struct CargoPackage {
    package: Package,
    deps: HashMap<String, PackageId>,
}

impl CargoPackage {
    pub fn name(&self) -> String {
        self.package.name.to_string()
    }

    pub fn version(&self) -> Version {
        self.package.version.clone()
    }

    pub fn crate_path(&self) -> PathBuf {
        self.package.manifest_path.as_std_path().parent().unwrap().to_path_buf()
    }

    pub fn dep(&self, name: &str) -> Option<&PackageId> {
        self.deps.get(name)
    }
}
