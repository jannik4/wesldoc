use crate::source_map::SourceMap;
use crate::{build_expression, map};
use std::collections::HashMap;
use wesl::syntax;
use wesl_docs::*;

pub fn build_attributes(
    attributes: &[syntax::Attribute],
    source_map: &SourceMap,
    module_path: &[String],
    dependencies: &HashMap<String, Version>,
) -> Vec<Attribute> {
    attributes
        .iter()
        .filter_map(|attr| build_attribute(attr, source_map, module_path, dependencies))
        .collect()
}

fn build_attribute(
    attr: &syntax::Attribute,
    source_map: &SourceMap,
    module_path: &[String],
    dependencies: &HashMap<String, Version>,
) -> Option<Attribute> {
    Some(match attr {
        syntax::Attribute::Align(expr) => Attribute::Align(build_expression(
            expr,
            source_map,
            module_path,
            dependencies,
        )),
        syntax::Attribute::Binding(expr) => Attribute::Binding(build_expression(
            expr,
            source_map,
            module_path,
            dependencies,
        )),
        syntax::Attribute::BlendSrc(expr) => Attribute::BlendSrc(build_expression(
            expr,
            source_map,
            module_path,
            dependencies,
        )),
        syntax::Attribute::Builtin(builtin_value) => Attribute::Builtin(map(builtin_value)),
        syntax::Attribute::Const => Attribute::Const,
        syntax::Attribute::Diagnostic(diagnostic_attribute) => Attribute::Diagnostic {
            severity: map(&diagnostic_attribute.severity),
            rule: diagnostic_attribute.rule.clone(),
        },
        syntax::Attribute::Group(expr) => Attribute::Group(build_expression(
            expr,
            source_map,
            module_path,
            dependencies,
        )),
        syntax::Attribute::Id(expr) => Attribute::Id(build_expression(
            expr,
            source_map,
            module_path,
            dependencies,
        )),
        syntax::Attribute::Interpolate(interpolate_attribute) => Attribute::Interpolate {
            ty: map(&interpolate_attribute.ty),
            sampling: interpolate_attribute.sampling.as_ref().map(map),
        },
        syntax::Attribute::Invariant => Attribute::Invariant,
        syntax::Attribute::Location(expr) => Attribute::Location(build_expression(
            expr,
            source_map,
            module_path,
            dependencies,
        )),
        syntax::Attribute::MustUse => Attribute::MustUse,
        syntax::Attribute::Size(expr) => Attribute::Size(build_expression(
            expr,
            source_map,
            module_path,
            dependencies,
        )),
        syntax::Attribute::WorkgroupSize(workgroup_size_attribute) => Attribute::WorkgroupSize {
            x: build_expression(
                &workgroup_size_attribute.x,
                source_map,
                module_path,
                dependencies,
            ),
            y: workgroup_size_attribute
                .y
                .as_ref()
                .map(|y| build_expression(y, source_map, module_path, dependencies)),
            z: workgroup_size_attribute
                .z
                .as_ref()
                .map(|z| build_expression(z, source_map, module_path, dependencies)),
        },
        syntax::Attribute::Vertex => Attribute::Vertex,
        syntax::Attribute::Fragment => Attribute::Fragment,
        syntax::Attribute::Compute => Attribute::Compute,
        syntax::Attribute::Custom(custom_attribute) => Attribute::Custom {
            name: custom_attribute.name.clone(),
            arguments: custom_attribute.arguments.as_ref().map(|args| {
                args.iter()
                    .map(|arg| build_expression(arg, source_map, module_path, dependencies))
                    .collect()
            }),
        },

        // Conditional attributes are handled separately
        syntax::Attribute::If(_) | syntax::Attribute::Elif(_) | syntax::Attribute::Else => {
            return None;
        }
    })
}
