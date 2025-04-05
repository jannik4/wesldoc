use crate::source_map::SourceMap;
use wesl::syntax;
use wesl_docs::*;

// TODO: Naive approximation/guess, use span from wesl when available: https://github.com/wgsl-tooling-wg/wesl-rs/issues/60
pub fn calculate_span(decl: &syntax::GlobalDeclaration, source_map: &SourceMap) -> Option<Span> {
    let source = source_map.default_source()?;
    let name = match decl {
        syntax::GlobalDeclaration::Void => return None,
        syntax::GlobalDeclaration::Declaration(declaration) => &declaration.ident,
        syntax::GlobalDeclaration::TypeAlias(type_alias) => &type_alias.ident,
        syntax::GlobalDeclaration::Struct(struct_) => &struct_.ident,
        syntax::GlobalDeclaration::Function(function) => &function.ident,
        syntax::GlobalDeclaration::ConstAssert(_const_assert) => return None,
    }
    .to_string();

    for (idx, line) in source.lines().enumerate() {
        if line.contains(&name) {
            return Some(Span {
                line_start: idx + 1,
                line_end: idx + 4, // just assume 3 lines for now
            });
        }
    }

    None
}
