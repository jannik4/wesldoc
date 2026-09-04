use crate::ATTRIBUTE_CONDITIONAL;
use anyhow::Result;
use wgsl_parse::{SyntaxNode, syntax::*};

pub fn preprocess(mut syntax: TranslationUnit) -> Result<TranslationUnit> {
    syntax.remove_voids();
    build_conditionals(&mut syntax);

    Ok(syntax)
}

fn build_conditionals(syntax: &mut TranslationUnit) {
    build_conditionals_imports(&mut syntax.imports);
    // TODO: directives
    build_conditionals_decls(&mut syntax.global_declarations, None);
}

fn build_conditionals_imports<'a>(imports: impl IntoIterator<Item = &'a mut ImportStatement>) {
    let conditional_scope = &mut ConditionalScope::default();

    for import in imports {
        let cond = build_conditional(conditional_scope, import.attributes());
        if let Some(cond) = cond {
            let attr = Attribute::Custom(CustomAttribute {
                name: ATTRIBUTE_CONDITIONAL.to_string(),
                arguments: Some(vec![cond]),
            });
            import
                .attributes
                .push(AttributeNode::new(attr, Span::default()));
        }
    }
}

fn build_conditionals_decls<'a>(
    decls: impl IntoIterator<Item = &'a mut GlobalDeclarationNode>,
    parent: Option<ExpressionNode>,
) {
    let conditional_scope = &mut ConditionalScope::default();

    for decl in decls {
        // Build conditional and combine with parent conditional
        let cond = build_conditional(conditional_scope, decl.attributes());
        let cond = match (parent.clone(), cond) {
            (Some(parent), Some(cond)) => Some(Spanned::new(
                Expression::Binary(BinaryExpression {
                    operator: BinaryOperator::ShortCircuitAnd,
                    left: parent,
                    right: cond,
                }),
                Span::default(),
            )),
            (Some(parent), None) => Some(parent),
            (None, Some(cond)) => Some(cond),
            (None, None) => None,
        };

        match &mut **decl {
            // Recursively build conditionals for compound declarations
            GlobalDeclaration::Compound(compound) => {
                build_conditionals_decls(&mut compound.body, cond.clone());
            }
            // Handle struct members
            GlobalDeclaration::Struct(struct_) => {
                let conditional_scope = &mut ConditionalScope::default();
                for member in &mut struct_.members {
                    let cond = build_conditional(conditional_scope, member.attributes());
                    if let Some(cond) = cond {
                        let attr = Attribute::Custom(CustomAttribute {
                            name: ATTRIBUTE_CONDITIONAL.to_string(),
                            arguments: Some(vec![cond]),
                        });
                        member
                            .attributes
                            .push(AttributeNode::new(attr, Span::default()));
                    }
                }
            }
            // Handle function parameters
            GlobalDeclaration::Function(function) => {
                let conditional_scope = &mut ConditionalScope::default();
                for param in &mut function.parameters {
                    let cond = build_conditional(conditional_scope, param.attributes());
                    if let Some(cond) = cond {
                        let attr = Attribute::Custom(CustomAttribute {
                            name: ATTRIBUTE_CONDITIONAL.to_string(),
                            arguments: Some(vec![cond]),
                        });
                        param
                            .attributes
                            .push(AttributeNode::new(attr, Span::default()));
                    }
                }
            }
            _ => (),
        }

        // Push conditional to attributes
        if let Some(cond) = cond {
            let attr = Attribute::Custom(CustomAttribute {
                name: ATTRIBUTE_CONDITIONAL.to_string(),
                arguments: Some(vec![cond]),
            });
            push_attribute(decl, attr);
        }
    }
}

#[derive(Debug, Default)]
struct ConditionalScope {
    prev: Vec<ExpressionNode>,
}

fn build_conditional(
    scope: &mut ConditionalScope,
    attributes: &[AttributeNode],
) -> Option<ExpressionNode> {
    for attr in attributes {
        match attr.node() {
            Attribute::If(cond) => {
                let this = cond.clone();
                scope.prev.clear();
                scope.prev.push(this.clone());
                return Some(this);
            }
            Attribute::Elif(cond) => {
                let this = cond.clone();
                let combined = scope.prev.iter().fold(this.clone(), |acc, c| {
                    Spanned::new(
                        Expression::Binary(BinaryExpression {
                            operator: BinaryOperator::ShortCircuitAnd,
                            left: acc,
                            right: Spanned::new(
                                Expression::Unary(UnaryExpression {
                                    operator: UnaryOperator::LogicalNegation,
                                    operand: c.clone(),
                                }),
                                Span::default(),
                            ),
                        }),
                        Span::default(),
                    )
                });
                scope.prev.push(this);
                return Some(combined);
            }
            Attribute::Else => {
                return scope
                    .prev
                    .drain(..)
                    .map(|c| {
                        Spanned::new(
                            Expression::Unary(UnaryExpression {
                                operator: UnaryOperator::LogicalNegation,
                                operand: c,
                            }),
                            Span::default(),
                        )
                    })
                    .reduce(|a, b| {
                        Spanned::new(
                            Expression::Binary(BinaryExpression {
                                operator: BinaryOperator::ShortCircuitAnd,
                                left: a,
                                right: b,
                            }),
                            Span::default(),
                        )
                    });
            }
            _ => (),
        }
    }

    scope.prev.clear();
    None
}

fn push_attribute(decl: &mut GlobalDeclaration, attr: Attribute) {
    let attr = AttributeNode::new(attr, Span::default());
    match decl {
        GlobalDeclaration::Void => (),
        GlobalDeclaration::Declaration(declaration) => {
            declaration.attributes.push(attr);
        }
        GlobalDeclaration::TypeAlias(type_alias) => {
            type_alias.attributes.push(attr);
        }
        GlobalDeclaration::Struct(s) => {
            s.attributes.push(attr);
        }
        GlobalDeclaration::Function(function) => {
            function.attributes.push(attr);
        }
        GlobalDeclaration::ConstAssert(const_assert) => {
            const_assert.attributes.push(attr);
        }
        GlobalDeclaration::Compound(compound) => {
            compound.attributes.push(attr);
        }
    }
}
