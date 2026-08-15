use crate::{CompileOptions, compile_state::CompileState, map::map};
use std::collections::HashMap;
use wesl::{CompileResult, Mangler, ModulePath, SourceMap as _, syntax};
use wesldoc_ast::{DefinitionPath, Ident, ItemKind, Version};
use wgsl_parse::SyntaxNode;

pub struct Context<'a> {
    compiled: &'a CompileResult,
    exports: HashMap<(ModulePath, Ident), Ident>, // (path, item_name) -> rename.unwrap_or(item_name)

    module_path: ModulePath,
    dependencies: &'a HashMap<String, (String, Version)>,

    local: HashMap<String, ItemKind>,
    local_path: ModulePath,

    compile_options: &'a CompileOptions,
    compile_state: &'a CompileState,
}

impl Context<'_> {
    pub fn init<'a>(
        imports: &[syntax::ImportStatement],
        compiled: &'a CompileResult,
        module_path: ModulePath,
        dependencies: &'a HashMap<String, (String, Version)>,

        compile_options: &'a CompileOptions,
        compile_state: &'a CompileState,
    ) -> Context<'a> {
        // Warn if the source map is not found
        if compiled.sourcemap.is_none() {
            log::warn!("no source map found for module {module_path:?}");
        }

        // Collect exports
        let exports = collect_exports(imports);

        // Build local items
        let local = compiled
            .syntax
            .global_declarations
            .iter()
            .filter_map(|decl| {
                let (ident, kind) = match decl.node() {
                    syntax::GlobalDeclaration::Void => return None,
                    syntax::GlobalDeclaration::Compound(_) => {
                        panic!("compound should have been flattened")
                    }
                    syntax::GlobalDeclaration::Declaration(declaration) => match declaration.kind {
                        syntax::DeclarationKind::Const => (&declaration.ident, ItemKind::Constant),
                        syntax::DeclarationKind::Override => {
                            (&declaration.ident, ItemKind::Override)
                        }
                        syntax::DeclarationKind::Let => return None, // should be unreachable?
                        syntax::DeclarationKind::Var(_) => {
                            (&declaration.ident, ItemKind::GlobalVariable)
                        }
                    },
                    syntax::GlobalDeclaration::TypeAlias(type_alias) => {
                        (&type_alias.ident, ItemKind::TypeAlias)
                    }
                    syntax::GlobalDeclaration::Struct(struct_) => {
                        (&struct_.ident, ItemKind::Struct)
                    }
                    syntax::GlobalDeclaration::Function(function) => {
                        (&function.ident, ItemKind::Function)
                    }
                    syntax::GlobalDeclaration::ConstAssert(_const_assert) => return None,
                };

                if compiled
                    .sourcemap
                    .as_ref()
                    .and_then(|s| s.get_decl(ident.name().as_str()))
                    .is_none()
                {
                    Some((ident.name().to_string(), kind))
                } else {
                    None
                }
            })
            .collect();

        Context {
            compiled,
            exports,

            module_path,
            dependencies,

            local,
            local_path: ModulePath {
                origin: syntax::PathOrigin::Relative(0),
                components: Vec::new(),
            },

            compile_options,
            compile_state,
        }
    }

    pub fn at_path(&self, path: ModulePath) -> Context<'_> {
        Context {
            compiled: self.compiled,
            exports: self.exports.clone(),
            module_path: path,
            dependencies: self.dependencies,
            local: self.local.clone(),
            local_path: self.local_path.clone(),
            compile_options: self.compile_options,
            compile_state: self.compile_state,
        }
    }

    pub fn compiled(&self) -> &CompileResult {
        self.compiled
    }

    pub fn compile_options(&self) -> &CompileOptions {
        self.compile_options
    }

    pub fn compile_state(&self) -> &CompileState {
        self.compile_state
    }

    pub fn as_local(&self, decl: &syntax::GlobalDeclaration) -> Option<Ident> {
        let decl = match decl {
            syntax::GlobalDeclaration::Void => return None,
            syntax::GlobalDeclaration::Compound(_) => {
                panic!("compound should have been flattened")
            }
            syntax::GlobalDeclaration::Declaration(declaration) => &declaration.ident,
            syntax::GlobalDeclaration::TypeAlias(type_alias) => &type_alias.ident,
            syntax::GlobalDeclaration::Struct(struct_) => &struct_.ident,
            syntax::GlobalDeclaration::Function(function) => &function.ident,
            syntax::GlobalDeclaration::ConstAssert(_const_assert) => return None,
        };
        let name = map(decl);
        self.local.contains_key(&name.0).then_some(name)
    }

    pub fn get_source(&self) -> Option<&str> {
        self.compiled
            .sourcemap
            .as_ref()
            .and_then(|s| s.get_source(&self.module_path))
    }

    pub fn as_export(&self, decl: &syntax::GlobalDeclaration) -> Option<(ModulePath, &Ident)> {
        let decl = match decl {
            syntax::GlobalDeclaration::Void => return None,
            syntax::GlobalDeclaration::Compound(_) => {
                panic!("compound should have been flattened")
            }
            syntax::GlobalDeclaration::Declaration(declaration) => &declaration.ident,
            syntax::GlobalDeclaration::TypeAlias(type_alias) => &type_alias.ident,
            syntax::GlobalDeclaration::Struct(struct_) => &struct_.ident,
            syntax::GlobalDeclaration::Function(function) => &function.ident,
            syntax::GlobalDeclaration::ConstAssert(_const_assert) => return None,
        };

        // TODO: This assumes the escape mangler was used.
        let mangler = wesl::EscapeMangler;
        let (path, name) = mangler.unmangle(&decl.name())?;

        let local_name = self.exports.get(&(path.clone(), Ident(name)))?;

        Some((path, local_name))
    }

    pub fn resolve_reference(
        &self,
        target: ResolveTarget,
    ) -> Option<(Ident, ItemKind, DefinitionPath)> {
        let (name, kind, path) = self.get_decl(target)?;
        let def_path = match &path.origin {
            syntax::PathOrigin::Absolute => DefinitionPath::Absolute(path.components.clone()),
            syntax::PathOrigin::Relative(n) => {
                if self.module_path.components.len() < *n {
                    log::warn!(
                        "invalid relative path for type {} in module {}",
                        name,
                        self.module_path.components.join("/")
                    );
                    return None;
                } else {
                    let mut combined = self.module_path.components
                        [0..self.module_path.components.len() - n]
                        .to_vec();
                    combined.extend_from_slice(&path.components);
                    DefinitionPath::Absolute(combined)
                }
            }
            syntax::PathOrigin::Package(package) => match self.dependencies.get(package) {
                Some((package, version)) => DefinitionPath::Package(
                    package.clone(),
                    version.clone(),
                    path.components.to_vec(),
                ),
                None => {
                    log::warn!("dependency {package} not found");
                    return None;
                }
            },
        };
        Some((Ident(name.to_string()), kind, def_path))
    }

    fn get_decl(&self, target: ResolveTarget) -> Option<(&str, ItemKind, &ModulePath)> {
        if let Some((decl, kind)) = self.local.get_key_value(target.as_str()) {
            return Some((decl, *kind, &self.local_path));
        }

        if let Some(sourcemap) = self.compiled.sourcemap.as_ref() {
            match target {
                ResolveTarget::Name(name) => {
                    // TODO: This assumes the escape mangler was used.
                    // TODO: This does not work if multiple items with the same name existed before mangling.
                    let mangler = wesl::EscapeMangler;
                    let (mangled, kind) = mangled_item(self.compiled, |ident| {
                        mangler
                            .unmangle(ident)
                            .is_some_and(|(_, unmangled)| unmangled == name)
                    })?;

                    let (path, name) = sourcemap.get_decl(&mangled)?;
                    return Some((name, kind, path));
                }
                ResolveTarget::MaybeMangled(name) => {
                    let (_, kind) = mangled_item(self.compiled, |ident| ident == name)?;
                    let (path, name) = sourcemap.get_decl(name)?;
                    return Some((name, kind, path));
                }
            }
        }

        None
    }
}

pub enum ResolveTarget<'a> {
    /// Raw name, e.g. from doc comments.
    Name(&'a str),
    /// Identifier from the source code.
    MaybeMangled(&'a str),
}

impl ResolveTarget<'_> {
    pub fn as_str(&self) -> &str {
        match self {
            ResolveTarget::Name(name) => name,
            ResolveTarget::MaybeMangled(name) => name,
        }
    }
}

fn mangled_item(
    compiled: &CompileResult,
    mut f: impl FnMut(&str) -> bool,
) -> Option<(String, ItemKind)> {
    for decl in &compiled.syntax.global_declarations {
        let Some(ident) = decl.ident() else {
            continue;
        };
        if f(ident.name().as_str()) {
            return match decl.node() {
                syntax::GlobalDeclaration::Void => None,
                syntax::GlobalDeclaration::Compound(_) => {
                    panic!("compound should have been flattened")
                }
                syntax::GlobalDeclaration::Declaration(declaration) => match declaration.kind {
                    syntax::DeclarationKind::Const => Some(ItemKind::Constant),
                    syntax::DeclarationKind::Override => None,
                    syntax::DeclarationKind::Let => None, // should be unreachable?
                    syntax::DeclarationKind::Var(_) => Some(ItemKind::GlobalVariable),
                },
                syntax::GlobalDeclaration::TypeAlias(_) => Some(ItemKind::TypeAlias),
                syntax::GlobalDeclaration::Struct(_) => Some(ItemKind::Struct),
                syntax::GlobalDeclaration::Function(_) => Some(ItemKind::Function),
                syntax::GlobalDeclaration::ConstAssert(_const_assert) => None,
            }
            .map(|kind| (ident.name().to_string(), kind));
        }
    }

    None
}

// TODO: This assumes the re-exported items are defined in the re-exported module.
// This does not handle re-exports of re-exports correctly.
fn collect_exports(imports: &[syntax::ImportStatement]) -> HashMap<(ModulePath, Ident), Ident> {
    fn add_rec(
        exports: &mut HashMap<(ModulePath, Ident), Ident>,
        path: &ModulePath,
        content: &syntax::ImportContent,
    ) {
        match content {
            syntax::ImportContent::Item(import_item) => {
                exports.insert(
                    (path.clone(), map(&import_item.ident)),
                    map(&import_item
                        .rename
                        .clone()
                        .unwrap_or_else(|| import_item.ident.clone())),
                );
            }
            syntax::ImportContent::Collection(imports) => {
                for import in imports {
                    let mut path = path.clone();
                    path.components.extend(import.path.iter().cloned());
                    add_rec(exports, &path, &import.content);
                }
            }
        }
    }

    let mut exports = HashMap::new();
    for import in imports {
        let is_export = import
            .attributes
            .iter()
            .any(|attr| **attr == syntax::Attribute::Publish);
        if is_export && let Some(path) = &import.path {
            add_rec(&mut exports, path, &import.content);
        }
    }
    exports
}
