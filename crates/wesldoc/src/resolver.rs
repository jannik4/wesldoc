use crate::{
    Package,
    cargo::{CargoMetadata, CargoPackage},
};
use std::{borrow::Cow, cell::RefCell, collections::HashMap, rc::Rc};
use wesl::{
    FileResolver, ModulePath, ResolveError, Resolver,
    syntax::{ImportStatement, PathOrigin, TranslationUnit},
};

pub struct DocsResolver {
    this: FileResolver,
    dependencies: Dependencies,

    root_file_imports: RefCell<Option<Vec<ImportStatement>>>,
}

enum Dependencies {
    Explicit {
        dependencies: HashMap<String, (Package, FileResolver)>,
    },
    Auto {
        dependencies: RefCell<HashMap<String, (Package, FileResolver)>>,
        cargo_metadata: Rc<CargoMetadata>,
        this_cargo_package: Box<CargoPackage>,
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

            root_file_imports: RefCell::new(None),
        }
    }

    pub fn new_auto(
        this: &Package,
        cargo_metadata: Rc<CargoMetadata>,
        cargo_package: CargoPackage,
    ) -> Self {
        Self {
            this: FileResolver::new(&this.root),
            dependencies: Dependencies::Auto {
                dependencies: RefCell::new(HashMap::new()),
                cargo_metadata,
                this_cargo_package: Box::new(cargo_package),
            },

            root_file_imports: RefCell::new(None),
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

    pub fn take_root_file_imports(&self) -> Vec<ImportStatement> {
        self.root_file_imports.borrow_mut().take().unwrap()
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
                        this_cargo_package,
                    } => {
                        let mut dependencies = dependencies.borrow_mut();
                        if let Some((_, resolver)) = dependencies.get(package) {
                            return f(resolver, &path_absolute);
                        }

                        // Dependency not used yet, try to find it
                        let dep = Package::new_dependency(
                            this_cargo_package,
                            package,
                            None,
                            cargo_metadata,
                        )
                        .map_err(|err| {
                            ResolveError::ModuleNotFound(path.clone(), err.to_string())
                        })?;
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

    fn resolve_module(&self, path: &ModulePath) -> Result<TranslationUnit, ResolveError> {
        let source = self.resolve_source(path)?;
        let wesl = source.parse::<TranslationUnit>().map_err(|e| {
            wesl::Diagnostic::from(e)
                .with_module_path(path.clone(), self.display_name(path))
                .with_source(source.to_string())
        })?;

        let mut root_file_imports = self.root_file_imports.borrow_mut();
        if root_file_imports.is_none() {
            *root_file_imports = Some(wesl.imports.clone());
        }

        Ok(wesl)
    }

    fn display_name(&self, path: &ModulePath) -> Option<String> {
        self.resolve(path, |resolver, path| Ok(resolver.display_name(path)))
            .ok()?
    }
}
