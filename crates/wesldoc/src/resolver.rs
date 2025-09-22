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

    fn resolver_and_path(
        &self,
        path: &ModulePath,
    ) -> Result<(&FileResolver, ModulePath), ResolveError> {
        match &path.origin {
            PathOrigin::Absolute | PathOrigin::Relative(_) => Ok((&self.this, path.clone())),
            PathOrigin::Package(package) => {
                let Some(package) = self.dependencies.get(package) else {
                    return Err(ResolveError::ModuleNotFound(
                        path.clone(),
                        "package not found".to_string(),
                    ));
                };
                let path = ModulePath {
                    origin: PathOrigin::Absolute,
                    components: path.components.clone(),
                };
                Ok((package, path))
            }
        }
    }
}

impl Resolver for DocsResolver {
    fn resolve_source<'a>(&'a self, path: &ModulePath) -> Result<Cow<'a, str>, ResolveError> {
        let (resolver, path) = self.resolver_and_path(path)?;
        resolver.resolve_source(&path)
    }

    fn display_name(&self, path: &ModulePath) -> Option<String> {
        let (resolver, path) = self.resolver_and_path(path).ok()?;
        resolver.display_name(&path)
    }
}
