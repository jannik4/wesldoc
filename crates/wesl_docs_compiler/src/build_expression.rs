use crate::{Context, build_type, map};
use wesl::syntax;
use wesl_docs::*;

pub fn build_expression(expr: &syntax::Expression, ctx: &Context) -> Expression {
    match expr {
        syntax::Expression::Literal(lit) => Expression::Literal(map(lit)),
        syntax::Expression::Parenthesized(parenthesized_expression) => {
            Expression::Parenthesized(Box::new(build_expression(
                parenthesized_expression.expression.node(),
                ctx,
            )))
        }
        syntax::Expression::NamedComponent(_) => Expression::Unknown,
        syntax::Expression::Indexing(_) => Expression::Unknown,
        syntax::Expression::Unary(_) => Expression::Unknown,
        syntax::Expression::Binary(_) => Expression::Unknown,
        syntax::Expression::FunctionCall(_) => Expression::Unknown,
        syntax::Expression::TypeOrIdentifier(type_expression) => {
            Expression::TypeOrIdentifier(build_type(type_expression, ctx))
        }
    }
}
