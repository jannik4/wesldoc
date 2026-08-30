use crate::{CompileOptions, ResolveItemKind, Resolver, compile_state::CompileState};
use std::sync::Mutex;
use wesldoc_ast::{DefinitionPath, Ident, ItemKind};
use wgsl_parse::syntax::{ModulePath, TranslationUnit};

pub struct Context<'a, T> {
    resolver: Mutex<&'a mut dyn Resolver<PackageId = T>>,
    package_id: &'a T,
    path: &'a [String],

    syntax: &'a TranslationUnit,
    source: &'a str,

    compile_options: &'a CompileOptions,
    compile_state: &'a CompileState,
}

impl<'a, T> Context<'a, T> {
    pub fn init(
        resolver: &'a mut dyn Resolver<PackageId = T>,
        package_id: &'a T,
        path: &'a [String],

        syntax: &'a TranslationUnit,
        source: &'a str,

        compile_options: &'a CompileOptions,
        compile_state: &'a CompileState,
    ) -> Self {
        Self {
            resolver: Mutex::new(resolver),
            package_id,
            path,

            syntax,
            source,

            compile_options,
            compile_state,
        }
    }

    pub fn syntax(&self) -> &TranslationUnit {
        self.syntax
    }

    pub fn source(&self) -> &str {
        self.source
    }

    pub fn compile_options(&self) -> &CompileOptions {
        self.compile_options
    }

    pub fn compile_state(&self) -> &CompileState {
        self.compile_state
    }

    pub fn resolve_item(
        &self,
        path: &ModulePath,
        item_kind: ResolveItemKind,
    ) -> Option<(Ident, ItemKind, DefinitionPath)> {
        let items =
            self.resolver
                .lock()
                .unwrap()
                .resolve_item(self.package_id, self.path, path, item_kind);

        // TODO(no-comp): ...
        items
            .first()
            .map(|item| (item.name.clone(), item.kind, item.def_path.clone()))
    }
}
