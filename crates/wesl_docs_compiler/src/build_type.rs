use crate::source_map::SourceMap;
use std::collections::HashMap;
use wesl::syntax;
use wesl_docs::*;

pub fn build_type(
    ty: &syntax::TypeExpression,
    source_map: &SourceMap,
    module_path: &[String],
    dependencies: &HashMap<String, Version>,
) -> Type {
    let name = ty.ident.name().clone();

    match source_map.get_decl(&name) {
        Some((path, name)) => Type::Named {
            name: name.to_string(),
            def_path: match path.origin {
                syntax::PathOrigin::Absolute => {
                    Some(DefinitionPath::Absolute(path.components.clone()))
                }
                syntax::PathOrigin::Relative(n) => {
                    if module_path.len() < n + 1 {
                        println!(
                            "Warning: Invalid relative path for type {} in module {}",
                            name,
                            module_path.join("/")
                        );
                        None
                    } else {
                        let mut combined = module_path[1..module_path.len() - n].to_vec();
                        combined.extend_from_slice(&path.components);
                        Some(DefinitionPath::Absolute(combined))
                    }
                }
                syntax::PathOrigin::Package => match path.components.split_first() {
                    Some((dep, rest)) => match dependencies.get(dep) {
                        Some(version) => Some(DefinitionPath::Package(
                            dep.clone(),
                            version.clone(),
                            rest.to_vec(),
                        )),
                        None => {
                            println!("Warning: Dependency {} not found", dep,);
                            None
                        }
                    },
                    None => {
                        println!(
                            "Warning: Invalid package path for type {} in module {}",
                            name,
                            module_path.join("/")
                        );
                        None
                    }
                },
            },
        },
        None => Type::Named {
            name: type_to_string(ty),
            def_path: None,
        },
    }
}

fn type_to_string(ty: &syntax::TypeExpression) -> String {
    let mut name = ty.ident.name().clone();

    if let Some(template_args) = &ty.template_args {
        name.push('<');
        for (idx, arg) in template_args.iter().enumerate() {
            if idx > 0 {
                name.push_str(", ");
            }
            name.push_str(&expr_to_string(arg.expression.node()));
        }
        name.push('>');
    }

    name
}

#[expect(unused_variables)] // TODO: Remove this when implemented
fn expr_to_string(expr: &syntax::Expression) -> String {
    match expr {
        syntax::Expression::Literal(literal_expression) => "?".to_string(),
        syntax::Expression::Parenthesized(parenthesized_expression) => "?".to_string(),
        syntax::Expression::NamedComponent(named_component_expression) => "?".to_string(),
        syntax::Expression::Indexing(indexing_expression) => "?".to_string(),
        syntax::Expression::Unary(unary_expression) => "?".to_string(),
        syntax::Expression::Binary(binary_expression) => "?".to_string(),
        syntax::Expression::FunctionCall(function_call) => "?".to_string(),
        syntax::Expression::TypeOrIdentifier(type_expression) => type_to_string(type_expression),
    }
}
