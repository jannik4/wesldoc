use crate::context::Context;
use askama::Template;
use wesldoc_ast::{
    Attribute, BuiltinValue, Constant, DiagnosticSeverity, Expression, Function, GlobalVariable,
    InterpolationSampling, InterpolationType, ItemKind, Override, Struct, TypeAlias,
    TypeExpression,
};

#[derive(Template)]
#[template(path = "source.html")]
pub struct SourceTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub title: &'a str,
    pub source: &'a str,
    #[expect(unused)] // Only used as a flag in the template
    pub is_source_view: (),
}

#[derive(Template)]
#[template(path = "overview.html")]
pub struct OverviewTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub title: &'a str,
}

#[derive(Template)]
#[template(path = "constant.html")]
pub struct ConstantTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub title: &'a str,
    pub constants: &'a [Constant],
}

#[derive(Template)]
#[template(path = "override.html")]
pub struct OverrideTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub title: &'a str,
    pub overrides: &'a [Override],
}

#[derive(Template)]
#[template(path = "global_variable.html")]
pub struct GlobalVariableTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub title: &'a str,
    pub variables: &'a [GlobalVariable],
}

#[derive(Template)]
#[template(path = "struct.html")]
pub struct StructTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub title: &'a str,
    pub structs: &'a [Struct],
}

#[derive(Template)]
#[template(path = "function.html")]
pub struct FunctionTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub title: &'a str,
    pub functions: &'a [Function],
}

#[derive(Template)]
#[template(path = "type_alias.html")]
pub struct TypeAliasTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub title: &'a str,
    pub type_aliases: &'a [TypeAlias],
}

#[derive(Template)]
#[template(path = "render_type.html")]
pub struct RenderTypeTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub ty: &'a TypeExpression,
}

#[derive(Template)]
#[template(path = "render_expression.html")]
pub struct RenderExpressionTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub expr: &'a Expression,
}

#[derive(Template)]
#[template(path = "render_attribute.html")]
pub struct RenderAttributeTemplate<'a> {
    pub ctx: &'a Context<'a>,
    pub attr: &'a Attribute,
}

fn show_function_inline(function: &Function) -> bool {
    function.parameters.len() <= 3
        && function
            .parameters
            .iter()
            .all(|p| p.attributes.is_empty() && p.conditional.is_none())
}

fn module_path_class(kind: &ItemKind, last: &bool) -> &'static str {
    if !*last {
        return "path";
    }

    match kind {
        ItemKind::Module => "module",
        ItemKind::Constant => "const",
        ItemKind::Override => "override",
        ItemKind::GlobalVariable => "var",
        ItemKind::Struct => "struct",
        ItemKind::Function => "fn",
        ItemKind::TypeAlias => "type",
    }
}

fn builtin_str(builtin: &BuiltinValue) -> &'static str {
    match builtin {
        BuiltinValue::VertexIndex => "vertex_index",
        BuiltinValue::InstanceIndex => "instance_index",
        BuiltinValue::Position => "position",
        BuiltinValue::FrontFacing => "front_facing",
        BuiltinValue::FragDepth => "frag_depth",
        BuiltinValue::SampleIndex => "sample_index",
        BuiltinValue::SampleMask => "sample_mask",
        BuiltinValue::LocalInvocationId => "local_invocation_id",
        BuiltinValue::LocalInvocationIndex => "local_invocation_index",
        BuiltinValue::GlobalInvocationId => "global_invocation_id",
        BuiltinValue::WorkgroupId => "workgroup_id",
        BuiltinValue::NumWorkgroups => "num_workgroups",
    }
}

fn severity_str(diagnostic: &DiagnosticSeverity) -> &'static str {
    match diagnostic {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Off => "off",
    }
}

fn interpolation_str(interpolation: &InterpolationType) -> &'static str {
    match interpolation {
        InterpolationType::Perspective => "perspective",
        InterpolationType::Linear => "linear",
        InterpolationType::Flat => "flat",
    }
}

fn sampling_str(sampling: &InterpolationSampling) -> &'static str {
    match sampling {
        InterpolationSampling::Center => "center",
        InterpolationSampling::Centroid => "centroid",
        InterpolationSampling::Sample => "sample",
        InterpolationSampling::First => "first",
        InterpolationSampling::Either => "either",
    }
}
