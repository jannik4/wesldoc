mod all_items;
mod index;
mod static_files;

use askama::Template;
use serde_json::Value;
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs::{self, File},
    ops::Deref,
    path::Path,
};
use wesl_docs::{
    Attribute, BuiltinValue, Constant, DefinitionPath, DiagnosticSeverity, DocComment, Expression,
    Function, GlobalVariable, Ident, InterpolationSampling, InterpolationType, ItemKind, Module,
    Span, Struct, TypeAlias, TypeExpression, Version, WeslDocs,
};

pub type Error = Box<dyn std::error::Error>;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub fn generate(docs: &WeslDocs, base_path: &Path) -> Result<()> {
    // Write static files
    static_files::write_static_files(base_path)?;

    let base_path = base_path.join(&docs.root.name);

    // Prepare directories
    fs::create_dir_all(&base_path)?;

    // Versions
    let existing_versions = existing_versions(&base_path)?;
    let is_latest =
        existing_versions
            .iter()
            .all(|version| match docs.version.cmp_precedence(version) {
                Ordering::Equal | Ordering::Greater => true,
                Ordering::Less => false,
            });
    let all_versions = {
        let mut versions = existing_versions.clone();
        versions.insert(docs.version.clone());

        let mut versions = versions.into_iter().collect::<Vec<_>>();
        versions.sort_by(|a, b| a.cmp_precedence(b).reverse());

        versions
    };

    // Gen docs
    gen_doc(docs, false, &base_path)?;
    if is_latest {
        gen_doc(docs, true, &base_path)?;
    }

    // Store versions
    let mut common = load_common_json(&base_path)?;
    common["versions"] = Value::Array(
        all_versions
            .iter()
            .map(|version| Value::String(version.to_string()))
            .collect(),
    );
    store_common_json(&base_path, &common)?;

    // Update index
    index::update(&base_path.join(".."))?;

    Ok(())
}

fn load_common_json(base_path: &Path) -> Result<Value> {
    let common_path = base_path.join("common.js");
    let source = if common_path.exists() {
        fs::read_to_string(&common_path)?
    } else {
        return Ok(Value::Object(Default::default()));
    };

    let source = source.trim();
    let source = source
        .trim_start_matches("window.DOCS_COMMON")
        .trim_start()
        .trim_start_matches('=')
        .trim_start();
    let source = source.trim_end_matches(';').trim_end();

    Ok(serde_json::de::from_str(source)?)
}

fn store_common_json(base_path: &Path, value: &Value) -> Result<()> {
    let common_path = base_path.join("common.js");
    let source = format!(
        "window.DOCS_COMMON = {};\n",
        serde_json::ser::to_string_pretty(value)?
    );
    fs::write(common_path, source)?;
    Ok(())
}

fn existing_versions(path: &Path) -> Result<HashSet<Version>> {
    let mut versions = HashSet::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(version) = Version::parse(path.file_name().unwrap().to_str().unwrap()) {
                versions.insert(version);
            }
        }
    }
    Ok(versions)
}

fn gen_doc(doc: &WeslDocs, build_as_latest: bool, base_path: &Path) -> Result<()> {
    let base_path = if build_as_latest {
        base_path.join("latest")
    } else {
        base_path.join(doc.version.to_string())
    };
    let base_path_docs = base_path.join(&doc.root.name);
    let base_path_src = base_path.join("src").join(&doc.root.name);

    // Prepare directories
    fs::remove_dir_all(&base_path).ok();
    fs::create_dir_all(&base_path_docs)?;
    fs::create_dir_all(&base_path_src)?;

    // Gen modules
    gen_module(
        &Base {
            doc,
            build_as_latest,
            is_source_view: false,
        },
        &ModulePath {
            segments: vec![(
                doc.root.name.clone(),
                "index.html".to_string(),
                ItemKind::Module,
            )],
            level: 0,
        },
        &doc.root,
        &base_path_docs,
        &base_path_src,
    )?;

    // Store items
    let items = all_items::all_items(doc);
    let source = format!(
        "window.DOCS_ITEMS = {};\n",
        serde_json::ser::to_string(&items)?
    );
    fs::write(base_path.join("items.js"), source)?;

    Ok(())
}

fn gen_module(
    base: &Base,
    module_path: &ModulePath,
    module: &Module,
    base_path_docs: &Path,
    base_path_src: &Path,
) -> Result<()> {
    if let Some(source) = &module.source {
        let template = SourceTemplate {
            base: &base.with_source_view(),
            title: &module.name,
            module_path,
            source,
        };
        let mut path = base_path_src.to_path_buf();
        path.set_extension("html");
        template.write_into(&mut File::create(path)?)?;
    }

    let template = OverviewTemplate {
        base,
        title: &module.name,
        module_path,
        module,
    };
    template.write_into(&mut File::create(base_path_docs.join("index.html"))?)?;

    for module in &module.modules {
        let module_path = module_path.extend(&module.name, "index.html", ItemKind::Module, true);

        let base_path_docs = base_path_docs.join(&module.name);
        fs::create_dir(&base_path_docs)?;

        let base_path_src = base_path_src.join(&module.name);
        fs::create_dir(&base_path_src)?;

        gen_module(base, &module_path, module, &base_path_docs, &base_path_src)?;
    }

    for (name, item) in &module.constants {
        let module_path = module_path.extend(name.to_string(), "#", ItemKind::Constant, false);
        let template = ConstantTemplate {
            base,
            title: &name.to_string(),
            module_path: &module_path,
            module,
            constants: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("const.{}.html", name)),
        )?)?;
    }

    for (name, item) in &module.global_variables {
        let module_path =
            module_path.extend(name.to_string(), "#", ItemKind::GlobalVariable, false);
        let template = GlobalVariableTemplate {
            base,
            title: &name.to_string(),
            module_path: &module_path,
            module,
            variables: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("var.{}.html", name)),
        )?)?;
    }

    for (name, item) in &module.structs {
        let module_path = module_path.extend(name.to_string(), "#", ItemKind::Struct, false);
        let template = StructTemplate {
            base,
            title: &name.to_string(),
            module_path: &module_path,
            module,
            structs: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("struct.{}.html", name)),
        )?)?;
    }

    for (name, item) in &module.functions {
        let module_path = module_path.extend(name.to_string(), "#", ItemKind::Function, false);
        let template = FunctionTemplate {
            base,
            title: &name.to_string(),
            module_path: &module_path,
            module,
            functions: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("fn.{}.html", name)),
        )?)?;
    }

    for (name, item) in &module.type_aliases {
        let module_path = module_path.extend(name.to_string(), "#", ItemKind::TypeAlias, false);
        let template = TypeAliasTemplate {
            base,
            title: &name.to_string(),
            module_path: &module_path,
            module,
            type_aliases: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("type.{}.html", name)),
        )?)?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct ModulePath {
    segments: Vec<(String, String, ItemKind)>,
    level: usize,
}

impl ModulePath {
    fn extend(
        &self,
        name: impl Into<String>,
        path: impl Into<String>,
        kind: ItemKind,
        is_child: bool,
    ) -> Self {
        Self {
            segments: self
                .segments
                .iter()
                .map(|(name, path, kind)| {
                    if is_child {
                        (name.clone(), format!("../{}", path), *kind)
                    } else {
                        (name.clone(), path.clone(), *kind)
                    }
                })
                .chain([(name.into(), path.into(), kind)])
                .collect(),
            level: if is_child { self.level + 1 } else { self.level },
        }
    }

    fn root_path(&self) -> String {
        (0..self.level + 3).map(|_| "../").collect::<String>()
    }

    fn source_href(&self, span: Option<Span>) -> String {
        let mut href = String::new();

        for _ in 0..self.level {
            href.push_str("../");
        }
        href.push_str("../src/");

        for (idx, (name, _, kind)) in self.segments.iter().enumerate() {
            let is_last = idx == self.segments.len() - 1;

            if is_last {
                if *kind == ItemKind::Module {
                    href.push_str(name);
                    href.push_str(".html");
                } else {
                    href.pop();
                    href.push_str(".html");
                }
            } else {
                href.push_str(name);
                href.push('/');
            }
        }

        if let Some(span) = span {
            href.push('#');
            href.push_str(&span.line_start.to_string());
            if span.line_start != span.line_end {
                href.push('-');
                href.push_str(&span.line_end.to_string());
            }
        }

        href
    }
}

#[derive(Template)]
#[template(path = "source.html")]
struct SourceTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    source: &'a str,
}

#[derive(Template)]
#[template(path = "overview.html")]
struct OverviewTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    module: &'a Module,
}

#[derive(Template)]
#[template(path = "constant.html")]
struct ConstantTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    module: &'a Module,
    constants: &'a [Constant],
}

#[derive(Template)]
#[template(path = "global_variable.html")]
struct GlobalVariableTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    module: &'a Module,
    variables: &'a [GlobalVariable],
}

#[derive(Template)]
#[template(path = "struct.html")]
struct StructTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    module: &'a Module,
    structs: &'a [Struct],
}

#[derive(Template)]
#[template(path = "function.html")]
struct FunctionTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    module: &'a Module,
    functions: &'a [Function],
}

#[derive(Template)]
#[template(path = "type_alias.html")]
struct TypeAliasTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    module: &'a Module,
    type_aliases: &'a [TypeAlias],
}

#[derive(Template)]
#[template(path = "render_type.html")]
struct RenderTypeTemplate<'a> {
    ty: &'a TypeExpression,
    module_path_level: usize,
}

fn render_type(ty: &TypeExpression, module_path_level: &usize) -> String {
    RenderTypeTemplate {
        ty,
        module_path_level: *module_path_level,
    }
    .to_string()
}

#[derive(Template)]
#[template(path = "render_expression.html")]
struct RenderExpressionTemplate<'a> {
    expr: &'a Expression,
    module_path_level: usize,
}

impl RenderExpressionTemplate<'_> {
    fn render_rec(&self, expr: &Expression) -> String {
        RenderExpressionTemplate {
            expr,
            module_path_level: self.module_path_level,
        }
        .to_string()
    }
}

fn render_expression(expr: &Expression, module_path_level: &usize) -> String {
    RenderExpressionTemplate {
        expr,
        module_path_level: *module_path_level,
    }
    .to_string()
}

#[derive(Template)]
#[template(path = "render_attribute.html")]
struct RenderAttributeTemplate<'a> {
    attr: &'a Attribute,
    module_path_level: usize,
}

fn render_attributes(
    attributes: &[Attribute],
    module_path_level: &usize,
    new_line_indent: Option<usize>,
) -> String {
    if attributes.is_empty() {
        return "".to_string();
    }

    let sep = match new_line_indent {
        Some(indent) => {
            let mut sep = String::new();
            sep.push('\n');
            for _ in 0..indent {
                sep.push(' ');
            }
            sep
        }
        None => " ".to_string(),
    };

    let mut result = String::new();
    for (idx, attr) in attributes.iter().enumerate() {
        if idx != 0 {
            if attributes.len() <= 3 {
                result.push(' ');
            } else {
                result.push_str(&sep);
            }
        }
        result.push_str(
            &RenderAttributeTemplate {
                attr,
                module_path_level: *module_path_level,
            }
            .to_string(),
        );
    }

    result.push_str(&sep);

    result
}

fn show_function_inline(function: &Function) -> bool {
    function.parameters.len() <= 3
        && function
            .parameters
            .iter()
            .all(|p| p.attributes.is_empty() && p.conditional.is_none())
}

fn def_path_href(
    name: &Ident,
    kind: &ItemKind,
    def_path: &DefinitionPath,
    module_path_level: &usize,
) -> String {
    let mut href = String::new();

    match def_path {
        DefinitionPath::Absolute(components) => {
            for _ in 0..*module_path_level {
                href.push_str("../");
            }
            for c in components {
                href.push_str(c);
                href.push('/');
            }
        }
        DefinitionPath::Package(dep, version, components) => {
            for _ in 0..*module_path_level + 3 {
                href.push_str("../");
            }
            href.push_str(dep);
            href.push('/');
            href.push_str(&version.to_string());
            href.push('/');
            href.push_str(dep);
            href.push('/');
            for c in components {
                href.push_str(c);
                href.push('/');
            }
        }
    }

    match *kind {
        ItemKind::Module => href.push_str("index.html"),
        ItemKind::Constant => href.push_str(&format!("const.{}.html", name.0)),
        ItemKind::GlobalVariable => href.push_str(&format!("var.{}.html", name.0)),
        ItemKind::Struct => href.push_str(&format!("struct.{}.html", name.0)),
        ItemKind::Function => href.push_str(&format!("fn.{}.html", name.0)),
        ItemKind::TypeAlias => href.push_str(&format!("type.{}.html", name.0)),
    }

    href
}

fn render_doc_comment(comment: Option<&DocComment>) -> String {
    let mut output = String::new();
    if let Some(comment) = comment {
        output.push_str(r#"<div class="comment">"#);
        let md = {
            let mut md = String::new();
            wesl_docs::md::html::push_html(&mut md, comment.unsafe_full.iter().cloned());
            ammonia::clean(&md)
        };
        output.push_str(&md);
        output.push_str(r#"</div>"#);
    }
    output
}

fn render_doc_comment_short(comment: Option<&DocComment>) -> String {
    let mut output = String::new();
    if let Some(comment) = comment {
        output.push_str(r#"<div class="comment-inline">"#);
        let md = {
            let mut md = String::new();
            wesl_docs::md::html::push_html(&mut md, comment.unsafe_short.iter().cloned());
            ammonia::clean(&md)
        };
        output.push_str(&md);
        output.push_str(r#"</div>"#);
    }
    output
}

struct Base<'a> {
    doc: &'a WeslDocs,
    build_as_latest: bool,
    is_source_view: bool,
}

impl Base<'_> {
    fn with_source_view(&self) -> Self {
        Self {
            doc: self.doc,
            build_as_latest: self.build_as_latest,
            is_source_view: true,
        }
    }
}

fn module_path_class(kind: &ItemKind, last: &bool) -> &'static str {
    if !*last {
        return "path";
    }

    match kind {
        ItemKind::Module => "module",
        ItemKind::Constant => "const",
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
