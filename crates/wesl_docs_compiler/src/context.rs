use std::collections::HashMap;
use wesl::{CompileResult, Mangler, ModulePath, SourceMap as _, syntax};
use wesl_docs::{DefinitionPath, Ident, ItemKind, Version};

pub struct Context<'a> {
    compiled: &'a CompileResult,

    module_path: &'a [String],
    dependencies: &'a HashMap<String, Version>,

    local: HashMap<String, ItemKind>,
    local_path: ModulePath,
}

impl Context<'_> {
    pub fn init<'a>(
        compiled: &'a CompileResult,
        module_path: &'a [String],
        dependencies: &'a HashMap<String, Version>,
    ) -> Context<'a> {
        // Warn if the source map is not found
        if compiled.sourcemap.is_none() {
            log::warn!("no source map found for module {:?}", module_path);
        }

        // Build local items
        let local = compiled
            .syntax
            .global_declarations
            .iter()
            .filter_map(|decl| {
                let (ident, kind) = match decl.node() {
                    syntax::GlobalDeclaration::Void => return None,
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

            module_path,
            dependencies,

            local,
            local_path: ModulePath {
                origin: syntax::PathOrigin::Relative(0),
                components: Vec::new(),
            },
        }
    }

    pub fn compiled(&self) -> &CompileResult {
        self.compiled
    }

    pub fn is_local(&self, decl: &syntax::GlobalDeclaration) -> bool {
        let decl = match decl {
            syntax::GlobalDeclaration::Void => return false,
            syntax::GlobalDeclaration::Declaration(declaration) => &declaration.ident,
            syntax::GlobalDeclaration::TypeAlias(type_alias) => &type_alias.ident,
            syntax::GlobalDeclaration::Struct(struct_) => &struct_.ident,
            syntax::GlobalDeclaration::Function(function) => &function.ident,
            syntax::GlobalDeclaration::ConstAssert(_const_assert) => return false,
        };
        self.local.contains_key(decl.name().as_str())
    }

    pub fn default_source(&self) -> Option<&str> {
        self.compiled
            .sourcemap
            .as_ref()
            .and_then(|s| s.get_default_source())
    }

    pub fn resolve_reference(
        &self,
        target: ResolveTarget,
    ) -> Option<(Ident, ItemKind, DefinitionPath)> {
        let (name, kind, path) = self.get_decl(target)?;
        let def_path = match path.origin {
            syntax::PathOrigin::Absolute => DefinitionPath::Absolute(path.components.clone()),
            syntax::PathOrigin::Relative(n) => {
                if self.module_path.len() < n + 1 {
                    log::warn!(
                        "invalid relative path for type {} in module {}",
                        name,
                        self.module_path.join("/")
                    );
                    return None;
                } else {
                    let mut combined = self.module_path[1..self.module_path.len() - n].to_vec();
                    combined.extend_from_slice(&path.components);
                    DefinitionPath::Absolute(combined)
                }
            }
            syntax::PathOrigin::Package => match path.components.split_first() {
                Some((dep, rest)) => match self.dependencies.get(dep) {
                    Some(version) => {
                        DefinitionPath::Package(dep.clone(), version.clone(), rest.to_vec())
                    }
                    None => {
                        log::warn!("dependency {} not found", dep,);
                        return None;
                    }
                },
                None => {
                    log::warn!(
                        "invalid package path for type {} in module {}",
                        name,
                        self.module_path.join("/")
                    );
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
