use crate::Package;
use std::{borrow::Cow, collections::HashMap};
use wesl::{FileResolver, ModulePath, ResolveError, Resolver, syntax::PathOrigin};

pub struct DocsResolver {
    this: FileResolver,
    dependencies: HashMap<String, FileResolver>,
}

impl DocsResolver {
    pub fn new(this: &Package, dependencies: &[Package]) -> Self {
        Self {
            this: FileResolver::new(&this.root),
            dependencies: dependencies
                .iter()
                .map(|dep| (dep.name.clone(), FileResolver::new(&dep.root)))
                .collect(),
        }
    }

    fn package_and_path(
        &self,
        path: &ModulePath,
    ) -> Result<(&FileResolver, ModulePath), ResolveError> {
        let Some(package) = path
            .components
            .first()
            .and_then(|p| self.dependencies.get(p))
        else {
            return Err(ResolveError::ModuleNotFound(
                path.clone(),
                "package not found".to_string(),
            ));
        };
        let path = ModulePath {
            origin: PathOrigin::Absolute,
            components: path.components[1..].to_vec(),
        };
        Ok((package, path))
    }
}

impl Resolver for DocsResolver {
    fn resolve_source<'a>(&'a self, path: &ModulePath) -> Result<Cow<'a, str>, ResolveError> {
        if path.origin.is_package() {
            let (package, path) = self.package_and_path(path)?;
            package.resolve_source(&path)
        } else {
            self.this.resolve_source(path)
        }
    }

    fn display_name(&self, path: &ModulePath) -> Option<String> {
        if path.origin.is_package() {
            let (package, path) = self.package_and_path(path).ok()?;
            package.display_name(&path)
        } else {
            self.this.display_name(path)
        }
    }
}
