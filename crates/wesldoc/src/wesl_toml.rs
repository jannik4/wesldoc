use crate::Result;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct WeslToml {
    pub package: WeslTomlPackage,
    #[serde(default)]
    pub dependencies: HashMap<String, WeslTomlDependency>,
}

impl WeslToml {
    pub fn validate(&self) -> Result<()> {
        if self.package.edition != "unstable_2025" {
            return Err("only edition 'unstable_2025' is supported".into());
        }

        match self.package.package_manager {
            Some(WeslTomlPackageManager::Cargo) | None => (),
            Some(WeslTomlPackageManager::Npm) => {
                return Err("npm package manager is not supported yet".into());
            }
        }

        if self.package.dependencies == Some(DependenciesAuto::Auto)
            && !self.dependencies.is_empty()
        {
            return Err(
                "cannot have both 'dependencies = \"auto\"' and explicit dependencies".into(),
            );
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct WeslTomlPackage {
    #[serde(default = "latest_known_edition")]
    pub edition: String,
    #[serde(default = "default_root")]
    pub root: PathBuf,
    #[serde(rename = "package-manager")]
    pub package_manager: Option<WeslTomlPackageManager>,
    #[serde(default)]
    pub dependencies: Option<DependenciesAuto>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum WeslTomlPackageManager {
    #[serde(rename = "cargo")]
    Cargo,
    #[serde(rename = "npm")]
    Npm,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum DependenciesAuto {
    #[serde(rename = "auto")]
    Auto,
}

#[derive(Debug, Deserialize)]
pub struct WeslTomlDependency {
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub path: Option<PathBuf>,
}

fn latest_known_edition() -> String {
    "unstable_2025".to_string()
}

fn default_root() -> PathBuf {
    "shaders".into()
}
