use crate::{CargoMetadata, Package};
use std::{borrow::Cow, cell::RefCell, collections::HashMap};
use wesl::{FileResolver, ModulePath, ResolveError, Resolver, syntax::PathOrigin};

pub struct DocsResolver {
    this: FileResolver,
    dependencies: Dependencies,
}

enum Dependencies {
    Explicit {
        dependencies: HashMap<String, (Package, FileResolver)>,
    },
    Auto {
        dependencies: RefCell<HashMap<String, (Package, FileResolver)>>,
        cargo_metadata: Box<CargoMetadata>,
    },
}

impl DocsResolver {
    pub fn new_explicit(this: &Package, dependencies: impl IntoIterator<Item = Package>) -> Self {
        Self {
            this: FileResolver::new(&this.root),
            dependencies: Dependencies::Explicit {
                dependencies: dependencies
                    .into_iter()
                    .map(|dep| {
                        let resolver = FileResolver::new(&dep.root);
                        (dep.local_name.clone(), (dep, resolver))
                    })
                    .collect(),
            },
        }
    }

    pub fn new_auto(this: &Package, cargo_metadata: CargoMetadata) -> Self {
        Self {
            this: FileResolver::new(&this.root),
            dependencies: Dependencies::Auto {
                dependencies: RefCell::new(HashMap::new()),
                cargo_metadata: Box::new(cargo_metadata),
            },
        }
    }

    pub fn resolved_dependencies(&self) -> Vec<Package> {
        match &self.dependencies {
            Dependencies::Explicit { dependencies } => {
                dependencies.values().map(|(pkg, _)| pkg.clone()).collect()
            }
            Dependencies::Auto { dependencies, .. } => dependencies
                .borrow()
                .values()
                .map(|(pkg, _)| pkg.clone())
                .collect(),
        }
    }

    fn resolve<T>(
        &self,
        path: &ModulePath,
        f: impl FnOnce(&FileResolver, &ModulePath) -> Result<T, ResolveError>,
    ) -> Result<T, ResolveError> {
        match &path.origin {
            PathOrigin::Absolute | PathOrigin::Relative(_) => Ok(f(&self.this, path)?),
            PathOrigin::Package(package) => {
                // Rebase the path to be absolute
                let path_absolute = ModulePath {
                    origin: PathOrigin::Absolute,
                    components: path.components.clone(),
                };

                // Look up the resolver for the package
                match &self.dependencies {
                    Dependencies::Explicit { dependencies } => {
                        let (_, resolver) = dependencies.get(package).ok_or_else(|| {
                            ResolveError::ModuleNotFound(
                                path.clone(),
                                "package not found".to_string(),
                            )
                        })?;
                        f(resolver, &path_absolute)
                    }
                    Dependencies::Auto {
                        dependencies,
                        cargo_metadata,
                    } => {
                        let mut dependencies = dependencies.borrow_mut();
                        if let Some((_, resolver)) = dependencies.get(package) {
                            return f(resolver, &path_absolute);
                        }

                        // Dependency not used yet, try to find it
                        let dep = Package::new_dependency(package, None, cargo_metadata).map_err(
                            |err| ResolveError::ModuleNotFound(path.clone(), err.to_string()),
                        )?;
                        let resolver = FileResolver::new(&dep.root);

                        let res = f(&resolver, &path_absolute);
                        dependencies.insert(dep.local_name.clone(), (dep, resolver));

                        Ok(res?)
                    }
                }
            }
        }
    }
}

impl Resolver for DocsResolver {
    fn resolve_source<'a>(&'a self, path: &ModulePath) -> Result<Cow<'a, str>, ResolveError> {
        self.resolve(path, |resolver, path| {
            resolver
                .resolve_source(path)
                .map(|source| Cow::Owned(source.into()))
        })
    }

    fn display_name(&self, path: &ModulePath) -> Option<String> {
        self.resolve(path, |resolver, path| Ok(resolver.display_name(path)))
            .ok()?
    }
}
