use crate::{Context, ResolveTarget, build_expression};
use wesl::syntax;
use wesldoc_ast::*;

pub fn build_type(ty: &syntax::TypeExpression, ctx: &Context) -> TypeExpression {
    let name = ty.ident.name().clone();

    match ctx.resolve_reference(ResolveTarget::MaybeMangled(&name)) {
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
