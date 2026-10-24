use anyhow::Result;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
pub struct WeslToml {
    #[serde(flatten)]
    pub package: Package,
    #[serde(default)]
    pub dependencies: Dependencies,
}

impl WeslToml {
    pub fn load(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = path.as_ref();
        if !path.is_file() {
            return Ok(None);
        }

        let this = toml::from_slice::<Self>(&fs::read(path)?)?;
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

        Ok(())
    }
}

impl Default for WeslToml {
    fn default() -> Self {
        Self {
            package: Package {
                edition: latest_known_edition(),
                root: default_root(),
                package_manager: None,
            },
            dependencies: Dependencies::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    #[serde(default = "latest_known_edition")]
    pub edition: String,
    #[serde(default = "default_root")]
    pub root: PathBuf,
    #[serde(rename = "package-manager")]
    pub package_manager: Option<PackageManager>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum PackageManager {
    #[serde(rename = "cargo")]
    Cargo,
    #[serde(rename = "npm")]
    Npm,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Dependencies {
    Auto(DependenciesAuto),
    Explicit(HashMap<String, Dependency>),
}

impl Default for Dependencies {
    fn default() -> Self {
        Dependencies::Auto(DependenciesAuto::Auto)
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum DependenciesAuto {
    #[serde(rename = "auto")]
    Auto,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
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
