use crate::{build_expression, source_map::SourceMap};
use std::collections::HashMap;
use wesl::syntax;
use wesl_docs::*;

pub fn build_type(
    ty: &syntax::TypeExpression,
    source_map: &SourceMap,
    module_path: &[String],
    dependencies: &HashMap<String, Version>,
) -> TypeExpression {
    let name = ty.ident.name().clone();

    match source_map.resolve_reference(&name, module_path, dependencies) {
        Some((name, kind, def_path)) => TypeExpression::Referenced {
            name,
            kind,
            def_path,
        },
        None => TypeExpression::TypeIdentifier {
            name: Ident(name),
            template_args: ty.template_args.as_ref().map(|args| {
                args.iter()
                    .map(|arg| {
                        build_expression(
                            arg.expression.node(),
                            source_map,
                            module_path,
                            dependencies,
                        )
                    })
                    .collect()
            }),
        },
    }
}
