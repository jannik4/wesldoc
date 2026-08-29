use crate::{
    build::BuildCache,
    cargo::CargoPackage,
    package::{Package, PackageId},
    wesl_toml::DependenciesAuto,
};
use anyhow::Result;
use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
};
use wesldoc_ast::{Conditional, DefinitionPath, ItemKind};
use wesldoc_compiler::{
    ResolvedItem, Resolver, ResolverResult, build_conditional::conditional_from_attributes,
};
use wgsl_parse::{SyntaxNode, syntax};

pub struct CompilePackageResolver<'a> {
    cache: &'a mut BuildCache,
    root_package: Arc<Package>,
    dependencies: Dependencies,
}

impl<'a> CompilePackageResolver<'a> {
    pub fn new(
        cache: &'a mut BuildCache,
        package: Package,
        cargo_package: &CargoPackage,
    ) -> Result<Self> {
        let dependencies = match package.wesl_toml.package.dependencies {
            Some(DependenciesAuto::Auto) => Dependencies::Auto {
                dependencies: HashMap::new(),
            },
            None => Dependencies::Explicit {
                dependencies: package
                    .wesl_toml
                    .dependencies
                    .iter()
                    .map(|(dep_key, dep)| {
                        Ok((
                            dep_key.clone(),
                            Arc::new(Package::new_dependency(
                                cargo_package,
                                dep_key,
                                Some(dep),
                                cache.cargo_metadata(),
                            )?),
                        ))
                    })
                    .collect::<Result<_>>()?,
            },
        };

        Ok(Self {
            cache,
            root_package: Arc::new(package),
            dependencies,
        })
    }

    fn get_syntax(
        &mut self,
        package_id: PackageId,
        path: &[String],
    ) -> Option<Arc<syntax::TranslationUnit>> {
        let wesl_package = self.cache.get_or_build(package_id).ok().flatten()?;

        // Navigate to the module specified by the path
        let mut module = &*wesl_package;
        for component in path {
            let submodule = module.submodules.iter().find(|m| m.name == *component)?;
            module = submodule;
        }
        let (syntax, _) = module.code.as_ref()?;

        Some(Arc::clone(syntax))
    }

    // TODO: detect infinite loops!!! (e.g. keep track of visited (package.id, path), break on cycle)
    fn resolve_in(
        &mut self,
        package: &Package,
        include_imports: bool,
        path: &[String],
        name: &str,
        results: &mut Vec<ResolvedItem>,
        condition: Conditional,
    ) {
        let Some(syntax) = self.get_syntax(package.id.clone(), path) else {
            return;
        };

        // Lookup name in imports/exports
        for import in &syntax.imports {
            let is_export = import.attributes.iter().any(|attr| attr.is_publish());
            if !include_imports && !is_export {
                continue;
            }

            self.resolve_import(
                package,
                import,
                path,
                &[name.to_string()],
                results,
                condition.clone(),
            );
        }

        // Lookup name in global declarations
        for decl in &syntax.global_declarations {
            let Some((decl_name, decl_kind)) = decl_info(decl) else {
                continue;
            };
            if *decl_name.name() != name {
                continue;
            }
            let decl_condition =
                conditional_from_attributes(decl.attributes()).unwrap_or(Conditional::True);

            results.push(ResolvedItem {
                kind: decl_kind,
                def_path: if package.id == self.root_package.id {
                    DefinitionPath::Absolute(path.to_vec())
                } else {
                    DefinitionPath::Package(
                        package.package_name.clone(),
                        package.version.clone(),
                        path.to_vec(),
                    )
                },
                conditional: Some(Conditional::And(
                    Box::new(condition.clone()),
                    Box::new(decl_condition),
                )),
            });
        }
    }

    fn resolve_import(
        &mut self,
        package: &Package,
        import: &syntax::ImportStatement,
        path: &[String],
        item_path_and_name: &[String],
        results: &mut Vec<ResolvedItem>,
        condition: Conditional,
    ) {
        let Some(import_path) = &import.path else {
            // TODO: Handle this
            return;
        };
        let import_condition =
            conditional_from_attributes(import.attributes()).unwrap_or(Conditional::True);

        let mut to_resolve = vec![(import_path.clone(), &import.content)];
        while let Some((mut import_path, content)) = to_resolve.pop() {
            match content {
                syntax::ImportContent::Item(import_item) => {
                    // Check name
                    match &import_item.rename {
                        Some(rename) => {
                            if *rename.name() != item_path_and_name[0] {
                                continue;
                            }
                        }
                        None => {
                            if *import_item.ident.name() != item_path_and_name[0] {
                                continue;
                            }
                        }
                    }

                    import_path
                        .components
                        .extend_from_slice(&item_path_and_name[..item_path_and_name.len() - 1]);
                    let name = item_path_and_name.last().unwrap();

                    // And condition with import's conditional
                    let condition = Conditional::And(
                        Box::new(condition.clone()),
                        Box::new(import_condition.clone()),
                    );

                    // Resolve
                    match import_path.origin {
                        syntax::PathOrigin::Absolute => {
                            let path = &import_path.components;
                            self.resolve_in(package, false, path, name, results, condition);
                        }
                        syntax::PathOrigin::Relative(n) => {
                            let to_keep = path.len().saturating_sub(n);
                            let path = path
                                .iter()
                                .take(to_keep)
                                .chain(&import_path.components)
                                .cloned()
                                .collect::<Vec<_>>();

                            self.resolve_in(package, false, &path, name, results, condition);
                        }
                        syntax::PathOrigin::Package(package_name) => {
                            let package = match &mut self.dependencies {
                                Dependencies::Explicit { dependencies } => {
                                    match dependencies.get(&package_name) {
                                        Some(pkg) => Arc::clone(pkg),
                                        None => {
                                            println!(
                                                "Warning: dependency '{}' not found",
                                                package_name
                                            );
                                            continue;
                                        }
                                    }
                                }
                                Dependencies::Auto { dependencies } => {
                                    match dependencies.entry(package_name.clone()) {
                                        Entry::Occupied(entry) => Arc::clone(entry.get()),
                                        Entry::Vacant(entry) => {
                                            let PackageId::Cargo(package_id) = &package.id else {
                                                continue;
                                            };
                                            let Some(this_cargo_package) =
                                                self.cache.cargo_metadata().package(package_id)
                                            else {
                                                continue;
                                            };

                                            // TODO: handle error?
                                            match Package::new_dependency(
                                                this_cargo_package,
                                                package_name,
                                                None,
                                                self.cache.cargo_metadata(),
                                            ) {
                                                Ok(pkg) => {
                                                    let pkg = Arc::new(pkg);
                                                    entry.insert(Arc::clone(&pkg));
                                                    pkg
                                                }
                                                Err(_) => continue,
                                            }
                                        }
                                    }
                                }
                            };
                            let path = &import_path.components;
                            self.resolve_in(&package, false, path, name, results, condition);
                        }
                    }
                }
                syntax::ImportContent::Collection(imports) => {
                    for import in imports {
                        let import_path = import_path.clone().join(import.path.iter().cloned());
                        to_resolve.push((import_path, &import.content));
                    }
                }
            }
        }
    }
}

impl Resolver for CompilePackageResolver<'_> {
    fn resolve_item(
        &mut self,
        path_from: &[String],
        item_path: &syntax::ModulePath,
        item_name: &str,
    ) -> Vec<ResolvedItem> {
        let mut results = Vec::new();

        match &item_path.origin {
            syntax::PathOrigin::Absolute => {
                self.resolve_in(
                    &Arc::clone(&self.root_package),
                    false,
                    &item_path.components,
                    item_name,
                    &mut results,
                    Conditional::True,
                );
            }
            syntax::PathOrigin::Relative(n) => {
                let include_imports = *n == 0 && item_path.components.is_empty();

                let to_keep = path_from.len().saturating_sub(*n);
                let path = path_from.iter().take(to_keep).cloned().collect::<Vec<_>>();

                self.resolve_in(
                    &Arc::clone(&self.root_package),
                    include_imports,
                    &path,
                    item_name,
                    &mut results,
                    Conditional::True,
                );
            }
            syntax::PathOrigin::Package(pkg) => {
                // Resolve as pkg as suffix as one of the imports
                if let Some(syntax) = self.get_syntax(self.root_package.id.clone(), path_from) {
                    for import in &syntax.imports {
                        let item_path_and_name = [pkg.clone()]
                            .into_iter()
                            .chain(item_path.components.iter().cloned())
                            .chain([item_name.to_string()])
                            .collect::<Vec<_>>();
                        self.resolve_import(
                            &Arc::clone(&self.root_package),
                            import,
                            path_from,
                            &item_path_and_name,
                            &mut results,
                            Conditional::True,
                        );
                    }
                }

                // TODO: resolve as pkg as dependency
                // TODO: if one of the imports (or the "sum" of the imports) is "unconditional"
                //       this should not be considered, as this shadows any dependency usage?
            }
        }

        results
    }

    fn finish(self) -> ResolverResult {
        ResolverResult {
            version: self.root_package.version.clone(),
            dependencies: self
                .dependencies
                .into_iter()
                .map(|package| (package.package_name.clone(), package.version.clone()))
                .collect(),
        }
    }
}

// local_name -> package
enum Dependencies {
    Explicit {
        dependencies: HashMap<String, Arc<Package>>,
    },
    Auto {
        dependencies: HashMap<String, Arc<Package>>,
    },
}

impl Dependencies {
    fn into_iter(self) -> impl Iterator<Item = Arc<Package>> {
        match self {
            Dependencies::Explicit { dependencies } => dependencies.into_values(),
            Dependencies::Auto { dependencies } => dependencies.into_values(),
        }
    }
}

fn decl_info(decl: &syntax::GlobalDeclaration) -> Option<(&syntax::Ident, ItemKind)> {
    match decl {
        syntax::GlobalDeclaration::Void => None,
        syntax::GlobalDeclaration::Declaration(declaration) => Some((
            &declaration.ident,
            match declaration.kind {
                syntax::DeclarationKind::Const => ItemKind::Constant,
                syntax::DeclarationKind::Override => ItemKind::Override,
                syntax::DeclarationKind::Let => return None,
                syntax::DeclarationKind::Var(_) => ItemKind::GlobalVariable,
            },
        )),
        syntax::GlobalDeclaration::TypeAlias(type_alias) => {
            Some((&type_alias.ident, ItemKind::TypeAlias))
        }
        syntax::GlobalDeclaration::Struct(s) => Some((&s.ident, ItemKind::Struct)),
        syntax::GlobalDeclaration::Function(function) => {
            Some((&function.ident, ItemKind::Function))
        }
        syntax::GlobalDeclaration::ConstAssert(_) => None,
        syntax::GlobalDeclaration::Compound(_) => None,
    }
}
