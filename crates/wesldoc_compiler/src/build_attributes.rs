use crate::{Context, build_expression, map};
use wesl::syntax;
use wesldoc_ast::*;

pub fn build_attributes(attributes: &[syntax::AttributeNode], ctx: &Context) -> Vec<Attribute> {
    attributes
        .iter()
        .filter_map(|attr| build_attribute(attr, ctx))
        .collect()
}

fn build_attribute(attr: &syntax::Attribute, ctx: &Context) -> Option<Attribute> {
    Some(match attr {
        syntax::Attribute::Align(expr) => Attribute::Align(build_expression(expr, ctx)),
        syntax::Attribute::Binding(expr) => Attribute::Binding(build_expression(expr, ctx)),
        syntax::Attribute::BlendSrc(expr) => Attribute::BlendSrc(build_expression(expr, ctx)),
        syntax::Attribute::Builtin(builtin_value) => Attribute::Builtin(map(builtin_value)),
        syntax::Attribute::Const => Attribute::Const,
        syntax::Attribute::Diagnostic(diagnostic_attribute) => Attribute::Diagnostic {
            severity: map(&diagnostic_attribute.severity),
            rule: diagnostic_attribute.rule.clone(),
        },
        syntax::Attribute::Group(expr) => Attribute::Group(build_expression(expr, ctx)),
        syntax::Attribute::Id(expr) => Attribute::Id(build_expression(expr, ctx)),
        syntax::Attribute::Interpolate(interpolate_attribute) => Attribute::Interpolate {
            ty: map(&interpolate_attribute.ty),
            sampling: interpolate_attribute.sampling.as_ref().map(map),
        },
        syntax::Attribute::Invariant => Attribute::Invariant,
        syntax::Attribute::Location(expr) => Attribute::Location(build_expression(expr, ctx)),
        syntax::Attribute::MustUse => Attribute::MustUse,
        syntax::Attribute::Size(expr) => Attribute::Size(build_expression(expr, ctx)),
        syntax::Attribute::WorkgroupSize(workgroup_size_attribute) => Attribute::WorkgroupSize {
            x: build_expression(&workgroup_size_attribute.x, ctx),
            y: workgroup_size_attribute
                .y
                .as_ref()
                .map(|y| Box::new(build_expression(y, ctx))),
            z: workgroup_size_attribute
                .z
                .as_ref()
                .map(|z| Box::new(build_expression(z, ctx))),
        },
        syntax::Attribute::Vertex => Attribute::Vertex,
        syntax::Attribute::Fragment => Attribute::Fragment,
        syntax::Attribute::Compute => Attribute::Compute,
        syntax::Attribute::Custom(custom_attribute) => Attribute::Custom {
            name: custom_attribute.name.clone(),
            arguments: custom_attribute
                .arguments
                .as_ref()
                .map(|args| args.iter().map(|arg| build_expression(arg, ctx)).collect()),
        },

        // Conditional attributes are handled separately
        syntax::Attribute::If(_) | syntax::Attribute::Elif(_) | syntax::Attribute::Else => {
            return None;
        }

        // Ignore for now
        wesl::syntax::Attribute::Publish => return None, // TODO: handle publish attribute
    })
}
