use crate::source_map::SourceMap;
use crate::{build_type, map};
use std::collections::HashMap;
use wesl::syntax;
use wesl_docs::*;

pub fn build_expression(
    expr: &syntax::Expression,
    source_map: &SourceMap,
    module_path: &[String],
    dependencies: &HashMap<String, Version>,
) -> Expression {
    match expr {
        syntax::Expression::Literal(lit) => Expression::Literal(map(lit)),
        syntax::Expression::Parenthesized(parenthesized_expression) => {
            Expression::Parenthesized(Box::new(build_expression(
                parenthesized_expression.expression.node(),
                source_map,
                module_path,
                dependencies,
            )))
        }
        syntax::Expression::NamedComponent(_) => Expression::Unknown,
        syntax::Expression::Indexing(_) => Expression::Unknown,
        syntax::Expression::Unary(_) => Expression::Unknown,
        syntax::Expression::Binary(_) => Expression::Unknown,
        syntax::Expression::FunctionCall(_) => Expression::Unknown,
        syntax::Expression::TypeOrIdentifier(type_expression) => Expression::TypeOrIdentifier(
            build_type(type_expression, source_map, module_path, dependencies),
        ),
    }
}
