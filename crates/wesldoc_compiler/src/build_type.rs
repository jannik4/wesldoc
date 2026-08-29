use crate::{Context, build_expression};
use wesldoc_ast::*;
use wgsl_parse::syntax;

pub fn build_type(ty: &syntax::TypeExpression, ctx: &Context) -> TypeExpression {
    let name = ty.ident.name().clone();

    let mut path = ty.path.clone().unwrap_or_else(|| syntax::ModulePath {
        origin: syntax::PathOrigin::Relative(0),
        components: Vec::new(),
    });
    path.components.push(name.clone());

    match ctx.resolve_item(&path) {
        Some((name, kind, def_path)) => TypeExpression::Referenced {
            name,
            kind,
            def_path,
        },
        None => TypeExpression::TypeIdentifier {
            name: Ident(name),
            template_args: ty.template_args.as_ref().map(|args| {
                args.iter()
                    .map(|arg| build_expression(&arg.expression, ctx))
                    .collect()
            }),
        },
    }
}
