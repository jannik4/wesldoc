use crate::Result;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use wesldoc_ast::Version;

#[derive(Debug, Clone)]
pub struct Package {
    pub version: Version,
    pub root: Module,
}

impl Package {
    pub fn read_from_dir(
        name: impl Into<String>,
        version: Version,
        dir: impl AsRef<Path>,
    ) -> Result<Self> {
        Ok(Self {
            version,
            root: Module {
                name: name.into(),
                file: None,
                submodules: Module::build_submodules(dir.as_ref())?,
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub file: Option<File>,
    pub submodules: HashMap<String, Module>,
}

impl Module {
    fn build_submodules(dir: &Path) -> Result<HashMap<String, Module>> {
        let mut submodules = HashMap::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            let name = path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .replace('-', "_");

            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext == "wesl" || ext == "wgsl")
            {
                let source = fs::read_to_string(&path)?;

                let sub = submodules.entry(name).or_insert_with_key(|name| Module {
                    name: name.clone(),
                    file: None,
                    submodules: HashMap::new(),
                });
                sub.file = Some(File { source, path });
            } else if path.is_dir() {
                let sub = submodules.entry(name).or_insert_with_key(|name| Module {
                    name: name.clone(),
                    file: None,
                    submodules: HashMap::new(),
                });
                sub.submodules = Self::build_submodules(&path)?;
            }
        }

        Ok(submodules)
    }
}

#[derive(Debug, Clone)]
pub struct File {
    pub source: String,
    pub path: PathBuf,
}
