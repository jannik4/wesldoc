use crate::{
    build::{BuildCache, Dependencies},
    cargo::CargoMetadata,
    package::{Package, PackageId},
};
use anyhow::Result;
use either::Either;
use std::{
    collections::{HashSet, hash_map::Entry},
    sync::Arc,
};
use wesldoc_ast::{Conditional, DefinitionPath, Ident, ItemKind};
use wesldoc_compiler::{
    ResolveItemKind, ResolvedItem, Resolver, build_conditional::conditional_from_attributes,
};
use wgsl_parse::{SyntaxNode, syntax};

pub struct CompilePackageResolver<'a> {
    cache: &'a mut BuildCache,
    cargo_metadata: Arc<CargoMetadata>,
}

impl<'a> CompilePackageResolver<'a> {
    pub fn new(cache: &'a mut BuildCache, cargo_metadata: Arc<CargoMetadata>) -> Result<Self> {
        Ok(Self {
            cache,
            cargo_metadata,
        })
    }

    fn get_module_name(&mut self, package_id: PackageId, path: &[String]) -> Option<String> {
        let package = self.cache.get_or_build(package_id).ok().flatten()?;

        // Navigate to the module specified by the path
        let mut module = &*package.build;
        for component in path {
            let submodule = module.submodules.iter().find(|m| m.name == *component)?;
            module = submodule;
        }

        Some(module.name.clone())
    }

    fn get_syntax(
        &mut self,
        package_id: PackageId,
        path: &[String],
    ) -> Option<Arc<syntax::TranslationUnit>> {
        let package = self.cache.get_or_build(package_id).ok().flatten()?;

        // Navigate to the module specified by the path
        let mut module = &*package.build;
        for component in path {
            let submodule = module.submodules.iter().find(|m| m.name == *component)?;
            module = submodule;
        }
        let (syntax, _) = module.code.as_ref()?;

        Some(Arc::clone(syntax))
    }

    #[expect(clippy::too_many_arguments)]
    fn resolve_in(
        &mut self,
        root_package_id: &PackageId,
        package: &Package,
        item_path: &[String],
        item_kind: ResolveItemKind,
        include_imports: bool,
        visited: &mut HashSet<(PackageId, Vec<String>)>,
        results: &mut Vec<ResolvedItem>,
        condition: Conditional,
    ) {
        if !visited.insert((package.id.clone(), item_path.to_vec())) {
            // Already visited this (package.id, item_path), avoid infinite loop
            return;
        }

        let (prefix_path, name) = {
            match item_kind {
                ResolveItemKind::Declaration => (),
                ResolveItemKind::DeclarationOrModule => {
                    if let Some(mod_name) = self.get_module_name(package.id.clone(), item_path) {
                        results.push(ResolvedItem {
                            name: Ident(mod_name),
                            kind: ItemKind::Module,
                            def_path: if package.id == *root_package_id {
                                DefinitionPath::Absolute(item_path.to_vec())
                            } else {
                                DefinitionPath::Package(
                                    package.package_name.clone(),
                                    package.version.clone(),
                                    item_path.to_vec(),
                                )
                            },
                            conditional: None, // Modules always exist, so no conditional
                        });
                    }
                }
            }

            match item_path {
                [path @ .., name] => (path, name),
                [] => return, // No name to resolve
            }
        };

        let Some(syntax) = self.get_syntax(package.id.clone(), prefix_path) else {
            return;
        };

        // Lookup name in imports/exports
        for import in &syntax.imports {
            let is_export = import.attributes.iter().any(|attr| attr.is_publish());
            if include_imports || is_export {
                self.resolve_import(
                    root_package_id,
                    package,
                    import,
                    prefix_path,
                    std::slice::from_ref(name),
                    item_kind,
                    visited,
                    results,
                    condition.clone(),
                );
            }
        }

        // Lookup name in global declarations
        for decl in &syntax.global_declarations {
            let Some((decl_name, decl_kind)) = decl_info(decl) else {
                continue;
            };
            if *decl_name.name() != *name {
                continue;
            }
            let decl_condition =
                conditional_from_attributes(decl.attributes()).unwrap_or(Conditional::True);

            results.push(ResolvedItem {
                name: Ident(name.to_string()),
                kind: decl_kind,
                def_path: if package.id == *root_package_id {
                    DefinitionPath::Absolute(prefix_path.to_vec())
                } else {
                    DefinitionPath::Package(
                        package.package_name.clone(),
                        package.version.clone(),
                        prefix_path.to_vec(),
                    )
                },
                conditional: Some(Conditional::And(
                    Box::new(condition.clone()),
                    Box::new(decl_condition),
                )),
            });
        }
    }

    #[expect(clippy::too_many_arguments)]
    fn resolve_import(
        &mut self,
        root_package_id: &PackageId,
        package: &Package,
        import: &syntax::ImportStatement,
        path: &[String],
        item_path: &[String],
        item_kind: ResolveItemKind,
        visited: &mut HashSet<(PackageId, Vec<String>)>,
        results: &mut Vec<ResolvedItem>,
        condition: Conditional,
    ) {
        let Some(import_path) = &import.path else {
            // TODO: Handle no import path
            // for example see: https://github.com/webgpu-tools/wesl-rs/blob/3c94796ccf329076af6cf158727e5fa55eb3b82a/crates/wesl/src/import.rs#L383-L405
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
                            if *rename.name() != item_path[0] {
                                continue;
                            }
                        }
                        None => {
                            if *import_item.ident.name() != item_path[0] {
                                continue;
                            }
                        }
                    }

                    import_path
                        .components
                        .push(import_item.ident.name().to_string());
                    import_path.components.extend_from_slice(&item_path[1..]);

                    // And condition with import's conditional
                    let condition = Conditional::And(
                        Box::new(condition.clone()),
                        Box::new(import_condition.clone()),
                    );

                    // Resolve
                    match import_path.origin {
                        syntax::PathOrigin::Absolute => {
                            let path = &import_path.components;
                            self.resolve_in(
                                root_package_id,
                                package,
                                path,
                                item_kind,
                                false,
                                visited,
                                results,
                                condition,
                            );
                        }
                        syntax::PathOrigin::Relative(n) => {
                            let to_keep = path.len().saturating_sub(n);
                            let path = path
                                .iter()
                                .take(to_keep)
                                .chain(&import_path.components)
                                .cloned()
                                .collect::<Vec<_>>();

                            self.resolve_in(
                                root_package_id,
                                package,
                                &path,
                                item_kind,
                                false,
                                visited,
                                results,
                                condition,
                            );
                        }
                        syntax::PathOrigin::Package(package_name) => {
                            let package = match self
                                .resolve_dependency_package(&package.id, &package_name)
                            {
                                Some(pkg) => pkg,
                                None => {
                                    println!("Warning: dependency '{}' not found", package_name);
                                    continue;
                                }
                            };
                            let path = &import_path.components;
                            self.resolve_in(
                                root_package_id,
                                &package,
                                path,
                                item_kind,
                                false,
                                visited,
                                results,
                                condition,
                            );
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

    fn resolve_dependency_package(
        &mut self,
        package_from: &PackageId,
        dependency_name: &str,
    ) -> Option<Arc<Package>> {
        // TODO: handle error?
        let package = self
            .cache
            .get_or_build(package_from.clone())
            .ok()
            .flatten()?;

        Some(match &mut package.dependencies {
            Dependencies::Explicit { dependencies } => match dependencies.get(dependency_name) {
                Some(pkg) => Arc::clone(pkg),
                None => {
                    println!("Warning: dependency '{}' not found", dependency_name);
                    return None;
                }
            },
            Dependencies::Auto { dependencies } => {
                match dependencies.entry(dependency_name.to_string()) {
                    Entry::Occupied(entry) => Arc::clone(entry.get()),
                    Entry::Vacant(entry) => {
                        let this_package = match &package_from {
                            PackageId::Cargo(package_id) => {
                                Either::Left(self.cargo_metadata.package(package_id)?)
                            }
                            PackageId::Path(path, _) => Either::Right(&**path),
                        };

                        // TODO: handle error?
                        let pkg = Package::new_dependency(
                            this_package,
                            dependency_name,
                            None,
                            &self.cargo_metadata,
                        )
                        .ok()?;
                        let pkg = Arc::new(pkg);
                        entry.insert(Arc::clone(&pkg));
                        pkg
                    }
                }
            }
        })
    }
}

impl Resolver for CompilePackageResolver<'_> {
    type PackageId = PackageId;

    fn resolve_item(
        &mut self,
        package_from: &Self::PackageId,
        path_from: &[String],
        item_path: &syntax::ModulePath,
        item_kind: ResolveItemKind,
    ) -> Vec<ResolvedItem> {
        let mut results = Vec::new();

        let root_package_id = package_from;

        // TODO: handle error?
        let package_from = match self.cache.get_or_build(package_from.clone()) {
            Ok(Some(pkg)) => Arc::clone(&pkg.package),
            _ => return results,
        };

        match &item_path.origin {
            syntax::PathOrigin::Absolute => {
                self.resolve_in(
                    root_package_id,
                    &package_from,
                    &item_path.components,
                    item_kind,
                    false,
                    &mut HashSet::new(),
                    &mut results,
                    Conditional::True,
                );
            }
            syntax::PathOrigin::Relative(n) => {
                let include_imports = *n == 0 && item_path.components.len() == 1;

                let to_keep = path_from.len().saturating_sub(*n);
                let path = path_from
                    .iter()
                    .take(to_keep)
                    .chain(item_path.components.iter())
                    .cloned()
                    .collect::<Vec<_>>();

                self.resolve_in(
                    root_package_id,
                    &package_from,
                    &path,
                    item_kind,
                    include_imports,
                    &mut HashSet::new(),
                    &mut results,
                    Conditional::True,
                );
            }
            syntax::PathOrigin::Package(pkg) => {
                // Resolve using pkg as suffix as one of the imports, so not really a "package path"
                if let Some(syntax) = self.get_syntax(package_from.id.clone(), path_from) {
                    for import in &syntax.imports {
                        let item_path = [pkg.clone()]
                            .into_iter()
                            .chain(item_path.components.iter().cloned())
                            .collect::<Vec<_>>();
                        self.resolve_import(
                            root_package_id,
                            &package_from,
                            import,
                            path_from,
                            &item_path,
                            item_kind,
                            &mut HashSet::new(),
                            &mut results,
                            Conditional::True,
                        );
                    }
                }

                // TODO: if one of the imports (or the "sum" of the imports) from above is
                //                "unconditional" below should not be considered, as this shadows
                //                any dependency usage?

                // Resolve using pkg as dependency
                if let Some(dep_pkg) = self.resolve_dependency_package(&package_from.id, pkg) {
                    self.resolve_in(
                        root_package_id,
                        &dep_pkg,
                        &item_path.components,
                        item_kind,
                        false,
                        &mut HashSet::new(),
                        &mut results,
                        Conditional::True,
                    );
                }
            }
        }

        results
    }

    fn resolve_dependency(
        &mut self,
        package_from: &Self::PackageId,
        dependency_name: &str,
    ) -> Option<Self::PackageId> {
        self.resolve_dependency_package(package_from, dependency_name)
            .map(|pkg| pkg.id.clone())
    }

    fn resolved_dependencies(
        &self,
        package_id: &Self::PackageId,
    ) -> Vec<(String, wesldoc_ast::Version)> {
        // TODO: handle error?
        let Some(pkg) = self.cache.get(package_id) else {
            return Vec::new();
        };

        pkg.dependencies
            .clone()
            .into_iter()
            .map(|package| (package.package_name.clone(), package.version.clone()))
            .collect()
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
