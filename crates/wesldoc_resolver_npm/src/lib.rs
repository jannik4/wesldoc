use anyhow::{Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
};
use wesldoc_resolver::{ResolverBackend, ResolverBackendPackage};

pub struct NpmResolverBackend {}

impl NpmResolverBackend {
    pub const NAME: &'static str = "npm";

    pub fn is_applicable(base_path: &Path) -> Result<bool> {
        Ok(fs::exists(base_path.join("package.json"))?)
    }

    pub fn resolve(_base_path: &Path) -> Result<Self> {
        bail!("npm resolver is not implemented yet");
    }
}

impl ResolverBackend for NpmResolverBackend {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn package(
        &self,
        _package_id: &wesldoc_resolver::package::PackageIdBackend,
    ) -> Option<&dyn ResolverBackendPackage> {
        todo!()
    }

    fn iter_packages(
        &self,
        _max_dependency_depth: usize,
    ) -> Box<dyn Iterator<Item = &dyn ResolverBackendPackage> + '_> {
        todo!()
    }
}

pub struct NpmPackage;

impl ResolverBackendPackage for NpmPackage {
    fn path(&self) -> PathBuf {
        todo!()
    }

    fn to_package(&self) -> Result<wesldoc_resolver::package::Package> {
        todo!()
    }

    fn get_dependency(&self, _name: &str) -> Option<&wesldoc_resolver::package::PackageIdBackend> {
        todo!()
    }
}
