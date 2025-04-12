use crate::{Context, build_type, calculate_span, map};
use wesl::syntax;
use wesldoc_ast::*;

pub fn build_expression(expr: &syntax::ExpressionNode, ctx: &Context) -> Expression {
    match expr.node() {
        syntax::Expression::Literal(lit) => Expression::Literal(map(lit)),
        syntax::Expression::Parenthesized(parenthesized_expression) => Expression::Parenthesized(
            Box::new(build_expression(&parenthesized_expression.expression, ctx)),
        ),
        syntax::Expression::NamedComponent(_) => {
            Expression::NotExpanded(calculate_span(expr.span().range(), ctx))
        }
        syntax::Expression::Indexing(_) => {
            Expression::NotExpanded(calculate_span(expr.span().range(), ctx))
        }
        syntax::Expression::Unary(_) => {
            Expression::NotExpanded(calculate_span(expr.span().range(), ctx))
        }
        syntax::Expression::Binary(_) => {
            Expression::NotExpanded(calculate_span(expr.span().range(), ctx))
        }
        syntax::Expression::FunctionCall(_) => {
            Expression::NotExpanded(calculate_span(expr.span().range(), ctx))
        }
        syntax::Expression::TypeOrIdentifier(type_expression) => {
            Expression::TypeOrIdentifier(build_type(type_expression, ctx))
        }
    }
}
