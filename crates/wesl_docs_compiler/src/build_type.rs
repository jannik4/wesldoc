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

    match source_map.get_decl(&name) {
        Some((name, kind, path)) => TypeExpression::Referenced {
            name: Ident(name.to_string()),
            kind,
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
