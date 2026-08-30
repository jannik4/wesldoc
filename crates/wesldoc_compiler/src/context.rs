use crate::{CompileOptions, ResolveItemKind, Resolver, compile_state::CompileState};
use std::sync::Mutex;
use wesldoc_ast::{DefinitionPath, Ident, ItemKind};
use wgsl_parse::syntax::{ModulePath, TranslationUnit};

pub struct Context<'a> {
    resolver: Mutex<&'a mut dyn Resolver>,
    path: &'a [String],

    syntax: &'a TranslationUnit,
    source: &'a str,

    compile_options: &'a CompileOptions,
    compile_state: &'a CompileState,
}

impl Context<'_> {
    pub fn init<'a>(
        resolver: &'a mut dyn Resolver,
        path: &'a [String],

        syntax: &'a TranslationUnit,
        source: &'a str,

        compile_options: &'a CompileOptions,
        compile_state: &'a CompileState,
    ) -> Context<'a> {
        Context {
            resolver: Mutex::new(resolver),
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
        let items = self
            .resolver
            .lock()
            .unwrap()
            .resolve_item(self.path, path, item_kind);

        // TODO(no-comp): ...
        items
            .first()
            .map(|item| (item.name.clone(), item.kind, item.def_path.clone()))
    }
}
