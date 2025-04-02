use crate::package::Package;
use std::{borrow::Cow, collections::HashMap, path::Path};
use wesl::{FileResolver, ModulePath, ResolveError, Resolver};

pub struct DocsResolver {
    packages: HashMap<String, Package>,
    files: FileResolver,
}

impl DocsResolver {
    pub fn new(packages: HashMap<String, Package>, base: impl AsRef<Path>) -> Self {
        Self {
            packages,
            files: FileResolver::new(base),
        }
    }
}

impl Resolver for DocsResolver {
    fn resolve_source<'a>(&'a self, path: &ModulePath) -> Result<Cow<'a, str>, ResolveError> {
        if !path.origin.is_package() {
            return self.files.resolve_source(path);
        }

        let Some(package) = path.components.first().and_then(|p| self.packages.get(p)) else {
            return Err(ResolveError::ModuleNotFound(
                path.clone(),
                "no package found".to_string(),
            ));
        };

        let mut module = &package.root;
        for component in path.components.iter().skip(1) {
            match module.submodules.get(component) {
                Some(submodule) => module = submodule,
                None => {
                    return Err(ResolveError::ModuleNotFound(
                        path.clone(),
                        format!(
                            "in module `{}`, no submodule named `{component}`",
                            module.name
                        ),
                    ));
                }
            }
        }

        match &module.file {
            Some(file) => Ok(file.source.as_str().into()),
            None => Ok("".into()),
        }
    }
}
