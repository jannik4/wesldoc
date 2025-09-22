use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct WeslToml {
    pub package: WeslTomlPackage,
    #[serde(default)]
    pub dependencies: HashMap<String, WeslTomlDependency>,
}

#[derive(Debug, Deserialize)]
pub struct WeslTomlPackage {
    #[serde(default = "latest_known_edition")]
    pub edition: String,
    #[serde(default = "default_root")]
    pub root: PathBuf,
    #[serde(rename = "package-manager")]
    pub package_manager: Option<WeslTomlPackageManager>,
}

#[derive(Debug, Deserialize)]
pub enum WeslTomlPackageManager {
    #[serde(rename = "cargo")]
    Cargo,
    #[serde(rename = "npm")]
    Npm,
}

#[derive(Debug, Deserialize)]
pub struct WeslTomlDependency {
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

fn latest_known_edition() -> String {
    "unstable_2025".to_string()
}

fn default_root() -> PathBuf {
    "shaders".into()
}
