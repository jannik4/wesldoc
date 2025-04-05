use std::collections::HashSet;
use wesl::{BasicSourceMap, ModulePath, SourceMap as _, syntax};

pub struct SourceMap<'a> {
    inner: Option<&'a BasicSourceMap>,
    local: HashSet<String>,
    local_path: ModulePath,
}

impl SourceMap<'_> {
    pub fn new(inner: Option<&BasicSourceMap>) -> SourceMap<'_> {
        SourceMap {
            inner,
            local: HashSet::new(),
            local_path: ModulePath {
                origin: syntax::PathOrigin::Relative(0),
                components: Vec::new(),
            },
        }
    }

    pub fn insert_local(&mut self, decl: &str) {
        if self.get_decl(decl).is_none() {
            self.local.insert(decl.to_string());
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
        self.local.contains(decl.name().as_str())
    }

    pub fn get_decl(&self, decl: &str) -> Option<(&ModulePath, &str)> {
        if let Some(decl) = self.local.get(decl) {
            return Some((&self.local_path, decl));
        }

        if let Some(inner) = self.inner {
            return inner.get_decl(decl);
        }

        None
    }

    pub fn default_source(&self) -> Option<&str> {
        self.inner.and_then(|s| s.get_default_source())
    }
}
