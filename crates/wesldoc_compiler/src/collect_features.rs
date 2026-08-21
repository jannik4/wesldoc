use crate::{ATTRIBUTE_CONDITIONAL, Context};
use wesldoc_ast::IndexSet;
use wgsl_parse::syntax::*;

pub fn collect_features(ctx: &Context) -> IndexSet<String> {
    let mut features = IndexSet::new();

    for directive in &ctx.syntax().global_directives {
        collect_from_global_directive(directive, &mut features);
    }
    for decl in &ctx.syntax().global_declarations {
        collect_from_global_declaration(decl, &mut features);
    }

    features
}

fn collect_from_global_directive(directive: &GlobalDirective, features: &mut IndexSet<String>) {
    match directive {
        GlobalDirective::Diagnostic(diagnostic) => {
            collect_from_attributes(&diagnostic.attributes, features);
        }
        GlobalDirective::Enable(enable) => {
            collect_from_attributes(&enable.attributes, features);
        }
        GlobalDirective::Requires(requires) => {
            collect_from_attributes(&requires.attributes, features);
        }
    }
}

fn collect_from_global_declaration(decl: &GlobalDeclaration, features: &mut IndexSet<String>) {
    match decl {
        GlobalDeclaration::Void => (),
        GlobalDeclaration::Compound(compound) => {
            for decl in &compound.body {
                collect_from_global_declaration(decl, features);
            }
        }
        GlobalDeclaration::Declaration(declaration) => {
            collect_from_attributes(&declaration.attributes, features);
        }
        GlobalDeclaration::TypeAlias(type_alias) => {
            collect_from_attributes(&type_alias.attributes, features);
        }
        GlobalDeclaration::Struct(struct_) => {
            collect_from_attributes(&struct_.attributes, features);
            for member in &struct_.members {
                collect_from_attributes(&member.attributes, features);
            }
        }
        GlobalDeclaration::Function(function) => {
            collect_from_attributes(&function.attributes, features);
            for param in &function.parameters {
                collect_from_attributes(&param.attributes, features);
            }

            // TODO: collect from function body
        }
        GlobalDeclaration::ConstAssert(const_assert) => {
            collect_from_attributes(&const_assert.attributes, features);
        }
    }
}

fn collect_from_attributes(attributes: &Attributes, features: &mut IndexSet<String>) {
    for attr in attributes {
        // Attribute::If(spanned) => collect_from_cond(spanned.node(), features),
        // Attribute::Elif(spanned) => collect_from_cond(spanned.node(), features),
        if let Attribute::Custom(CustomAttribute { name, arguments }) = attr.node() {
            if name != ATTRIBUTE_CONDITIONAL {
                continue;
            }
            let Some(args) = arguments.as_ref() else {
                continue;
            };
            if args.len() != 1 {
                continue;
            }
            collect_from_cond(&args[0], features);
        }
    }
}

fn collect_from_cond(expr: &Expression, features: &mut IndexSet<String>) {
    match expr {
        Expression::Parenthesized(paren) => collect_from_cond(paren.expression.node(), features),
        Expression::Unary(unary) if unary.operator == UnaryOperator::LogicalNegation => {
            collect_from_cond(unary.operand.node(), features);
        }
        Expression::Binary(binary) => match binary.operator {
            BinaryOperator::ShortCircuitOr | BinaryOperator::ShortCircuitAnd => {
                collect_from_cond(binary.left.node(), features);
                collect_from_cond(binary.right.node(), features);
            }
            _ => (),
        },
        Expression::TypeOrIdentifier(type_or_ident) => {
            features.insert(type_or_ident.ident.name().clone());
        }
        _ => (),
    }
}
