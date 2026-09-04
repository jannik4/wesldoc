use crate::{CompileOptions, Resolver, compile_state::CompileState};
use std::sync::Mutex;
use wesldoc_ast::{DefinitionPath, Ident, ItemKind};
use wesldoc_resolver::{ResolveItemKind, package::PackageId};
use wgsl_parse::syntax::{ModulePath, TranslationUnit};

pub struct Context<'a, 'b> {
    resolver: Mutex<&'a mut Resolver<'b>>,
    package_id: &'a PackageId,
    path: &'a [String],

    syntax: &'a TranslationUnit,
    source: &'a str,

    compile_options: &'a CompileOptions,
    compile_state: &'a CompileState,
}

impl<'a, 'b> Context<'a, 'b> {
    pub fn init(
        resolver: &'a mut Resolver<'b>,
        package_id: &'a PackageId,
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

        // TODO: Return all items not just the first one!
        items
            .first()
            .map(|item| (item.name.clone(), item.kind, item.def_path.clone()))
    }
}
