use crate::{ATTRIBUTE_CONDITIONAL, map};
use wesldoc_ast::*;
use wgsl_parse::syntax;

pub fn conditional_from_attributes(attributes: &[syntax::AttributeNode]) -> Option<Conditional> {
    attributes.iter().find_map(|attr| {
        let syntax::Attribute::Custom(syntax::CustomAttribute { name, arguments }) = attr.node()
        else {
            return None;
        };
        if name != ATTRIBUTE_CONDITIONAL {
            return None;
        }
        let args = arguments.as_ref()?;
        if args.len() != 1 {
            return None;
        }
        conditional_from_expr(&args[0])
    })
}

pub fn conditional_from_expr(expr: &syntax::Expression) -> Option<Conditional> {
    match expr {
        syntax::Expression::Literal(lit) => match lit {
            syntax::LiteralExpression::Bool(true) => Some(Conditional::True),
            syntax::LiteralExpression::Bool(false) => Some(Conditional::False),
            _ => {
                log::warn!("unsupported literal type for conditional: {lit:?}");
                None
            }
        },
        syntax::Expression::Parenthesized(paren) => conditional_from_expr(paren.expression.node()),
        syntax::Expression::Unary(unary) => match unary.operator {
            syntax::UnaryOperator::LogicalNegation => Some(Conditional::Not(Box::new(
                conditional_from_expr(unary.operand.node())?,
            ))),
            _ => {
                log::warn!(
                    "unsupported unary operator for conditional: {:?}",
                    unary.operator
                );
                None
            }
        },
        syntax::Expression::Binary(binary) => match binary.operator {
            syntax::BinaryOperator::ShortCircuitOr => Some(Conditional::Or(
                Box::new(conditional_from_expr(binary.left.node())?),
                Box::new(conditional_from_expr(binary.right.node())?),
            )),
            syntax::BinaryOperator::ShortCircuitAnd => Some(Conditional::And(
                Box::new(conditional_from_expr(binary.left.node())?),
                Box::new(conditional_from_expr(binary.right.node())?),
            )),
            _ => {
                log::warn!(
                    "unsupported binary operator for conditional: {:?}",
                    binary.operator
                );
                None
            }
        },
        syntax::Expression::TypeOrIdentifier(type_or_ident) => {
            if type_or_ident.template_args.is_some() {
                log::warn!(
                    "template arguments are not supported in conditionals: {type_or_ident:?}"
                );
                None
            } else {
                Some(Conditional::Feature(map(&type_or_ident.ident)))
            }
        }
        _ => {
            log::warn!("unsupported expression type for conditional: {expr:?}");
            None
        }
    }
}
