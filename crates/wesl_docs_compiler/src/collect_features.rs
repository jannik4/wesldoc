use crate::source_map::SourceMap;
use wesl::{CompileResult, syntax::*};
use wesl_docs::IndexSet;

// TODO: Collect all features, this is not complete

pub fn collect_features(compiled: &CompileResult, source_map: &SourceMap) -> IndexSet<String> {
    let mut features = IndexSet::new();

    for decl in &compiled.syntax.global_declarations {
        if !source_map.is_local(decl) {
            continue;
        }
        collect_from_global_declaration(decl, &mut features);
    }

    features
}

fn collect_from_global_declaration(decl: &GlobalDeclaration, features: &mut IndexSet<String>) {
    match decl {
        GlobalDeclaration::Void => (),
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
        }
        GlobalDeclaration::ConstAssert(_const_assert) => (),
    }
}

fn collect_from_attributes(attributes: &Attributes, features: &mut IndexSet<String>) {
    for attr in attributes {
        match attr.node() {
            Attribute::If(spanned) => collect_from_expr(spanned.node(), features),
            Attribute::Elif(spanned) => collect_from_expr(spanned.node(), features),
            _ => (),
        }
    }
}

fn collect_from_expr(expr: &Expression, features: &mut IndexSet<String>) {
    match expr {
        Expression::Parenthesized(paren) => collect_from_expr(paren.expression.node(), features),
        Expression::Unary(unary) => {
            if unary.operator == UnaryOperator::LogicalNegation {
                collect_from_expr(unary.operand.node(), features);
            }
        }
        Expression::Binary(binary) => match binary.operator {
            BinaryOperator::ShortCircuitOr | BinaryOperator::ShortCircuitAnd => {
                collect_from_expr(binary.left.node(), features);
                collect_from_expr(binary.right.node(), features);
            }
            _ => (),
        },
        Expression::TypeOrIdentifier(type_or_ident) => {
            features.insert(type_or_ident.ident.name().clone());
        }
        _ => (),
    }
}
