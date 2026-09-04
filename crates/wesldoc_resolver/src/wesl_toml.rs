use anyhow::{Result, bail};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
pub struct WeslToml {
    pub package: WeslTomlPackage,
    #[serde(default)]
    pub dependencies: HashMap<String, WeslTomlDependency>,
}

impl WeslToml {
    pub fn load(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = path.as_ref();
        if !path.is_file() {
            return Ok(None);
        }

        let this = toml::from_slice::<WeslToml>(&fs::read(path)?)?;
        this.validate()?;
        Ok(Some(this))
    }

    fn validate(&self) -> Result<()> {
        if self.package.edition != latest_known_edition() {
            wesldoc_report::warn!(
                "unrecognized edition in wesl.toml: {}",
                self.package.edition,
            );
        }

        if self.package.dependencies == Some(DependenciesAuto::Auto)
            && !self.dependencies.is_empty()
        {
            bail!("cannot have both 'dependencies = \"auto\"' and explicit dependencies");
        }

        Ok(())
    }
}

impl Default for WeslToml {
    fn default() -> Self {
        Self {
            package: WeslTomlPackage {
                edition: latest_known_edition(),
                root: default_root(),
                package_manager: None,
                dependencies: Some(DependenciesAuto::Auto),
            },
            dependencies: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct WeslTomlDependency {
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub path: Option<PathBuf>,
}

fn latest_known_edition() -> String {
    "2026_pre".to_string()
}

fn default_root() -> PathBuf {
    "src".into()
}
