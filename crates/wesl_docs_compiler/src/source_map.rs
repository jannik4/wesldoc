use std::collections::HashMap;
use wesl::{CompileResult, ModulePath, SourceMap as _, syntax};
use wesl_docs::ItemKind;

pub struct SourceMap<'a> {
    compiled: &'a CompileResult,
    local: HashMap<String, ItemKind>,
    local_path: ModulePath,
}

impl SourceMap<'_> {
    pub fn new(compiled: &CompileResult) -> SourceMap<'_> {
        SourceMap {
            compiled,
            local: HashMap::new(),
            local_path: ModulePath {
                origin: syntax::PathOrigin::Relative(0),
                components: Vec::new(),
            },
        }
    }

    pub fn insert_local(&mut self, decl: &str, kind: ItemKind) {
        if self.get_decl(decl).is_none() {
            self.local.insert(decl.to_string(), kind);
        }
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

    pub fn get_decl(&self, decl: &str) -> Option<(&str, ItemKind, &ModulePath)> {
        if let Some((decl, kind)) = self.local.get_key_value(decl) {
            return Some((decl, *kind, &self.local_path));
        }

        if let Some(inner) = self.compiled.sourcemap.as_ref() {
            let kind = item_kind_from_name(self.compiled, decl)?;
            let (path, name) = inner.get_decl(decl)?;
            return Some((name, kind, path));
        }

        None
    }

    pub fn default_source(&self) -> Option<&str> {
        self.compiled
            .sourcemap
            .as_ref()
            .and_then(|s| s.get_default_source())
    }
}

fn item_kind_from_name(compiled: &CompileResult, name: &str) -> Option<ItemKind> {
    for decl in &compiled.syntax.global_declarations {
        if decl
            .ident()
            .is_some_and(|decl| decl.name().as_str() == name)
        {
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
            };
        }
    }

    None
}
