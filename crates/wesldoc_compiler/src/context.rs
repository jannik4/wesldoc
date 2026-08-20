use crate::{CompileOptions, Resolver, compile_state::CompileState};
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

    pub fn resolve_item(&self, item: &ModulePath) -> Option<(Ident, ItemKind, DefinitionPath)> {
        let items = self.resolver.lock().unwrap().resolve_item(self.path, item);

        // TODO: ...
        let name = item.components.last()?.clone();
        items
            .first()
            .map(|item| (Ident(name), item.kind, item.def_path.clone()))
    }

    // pub fn resolve_reference(
    //     &self,
    //     target: ResolveTarget,
    // ) -> Option<(Ident, ItemKind, DefinitionPath)> {
    //     let (name, kind, path) = self.get_decl(target)?;
    //     let def_path = match &path.origin {
    //         syntax::PathOrigin::Absolute => DefinitionPath::Absolute(path.components.clone()),
    //         syntax::PathOrigin::Relative(n) => {
    //             if self.module_path.components.len() < *n {
    //                 log::warn!(
    //                     "invalid relative path for type {} in module {}",
    //                     name,
    //                     self.module_path.components.join("/")
    //                 );
    //                 return None;
    //             } else {
    //                 let mut combined = self.module_path.components
    //                     [0..self.module_path.components.len() - n]
    //                     .to_vec();
    //                 combined.extend_from_slice(&path.components);
    //                 DefinitionPath::Absolute(combined)
    //             }
    //         }
    //         syntax::PathOrigin::Package(package) => match self.dependencies.get(package) {
    //             Some((package, version)) => DefinitionPath::Package(
    //                 package.clone(),
    //                 version.clone(),
    //                 path.components.to_vec(),
    //             ),
    //             None => {
    //                 log::warn!("dependency {package} not found");
    //                 return None;
    //             }
    //         },
    //     };
    //     Some((Ident(name.to_string()), kind, def_path))
    // }
}
