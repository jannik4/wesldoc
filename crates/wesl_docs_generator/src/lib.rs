mod all_items;

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
    Binding, BuiltIn, Constant, Function, GlobalVariable, Interpolation, Module, Sampling, Struct,
    Type, Version, WeslDocs,
};

pub fn generate(docs: &WeslDocs, base_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

fn load_common_json(base_path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
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

fn store_common_json(base_path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let common_path = base_path.join("common.js");
    let source = format!(
        "window.DOCS_COMMON = {};\n",
        serde_json::ser::to_string_pretty(value)?
    );
    fs::write(common_path, source)?;
    Ok(())
}

fn existing_versions(path: &Path) -> Result<HashSet<Version>, Box<dyn std::error::Error>> {
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

fn gen_doc(
    doc: &WeslDocs,
    build_as_latest: bool,
    base_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_path = if build_as_latest {
        base_path.join("latest").join(&doc.root.name)
    } else {
        base_path.join(doc.version.to_string()).join(&doc.root.name)
    };

    // Prepare directories
    fs::remove_dir_all(&base_path).ok();
    fs::create_dir_all(&base_path)?;

    // Gen modules
    gen_module(
        &Base {
            doc,
            build_as_latest,
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
        &base_path,
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
    base_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let template = OverviewTemplate {
        base,
        title: &module.name,
        module_path,
        module,
    };
    template.write_into(&mut File::create(base_path.join("index.html"))?)?;

    for module in &module.modules {
        let module_path = module_path.extend(&module.name, "index.html", ItemKind::Module, true);

        let base_path = base_path.join(&module.name);
        fs::create_dir(&base_path)?;

        gen_module(base, &module_path, module, &base_path)?;
    }

    for constant in &module.constants {
        let module_path =
            module_path.extend(constant.name.to_string(), "#", ItemKind::Constant, false);
        let template = ConstantTemplate {
            base,
            title: &constant.name.to_string(),
            module_path: &module_path,
            module,
            constant,
        };
        template.write_into(&mut File::create(
            base_path.join(format!("const.{}.html", constant.name)),
        )?)?;
    }

    for var in &module.global_variables {
        let module_path =
            module_path.extend(var.name.to_string(), "#", ItemKind::GlobalVariable, false);
        let template = GlobalVariableTemplate {
            base,
            title: &var.name.to_string(),
            module_path: &module_path,
            module,
            var,
        };
        template.write_into(&mut File::create(
            base_path.join(format!("var.{}.html", var.name)),
        )?)?;
    }

    for struct_ in &module.structs {
        let module_path =
            module_path.extend(struct_.name.to_string(), "#", ItemKind::Struct, false);
        let template = StructTemplate {
            base,
            title: &struct_.name.to_string(),
            module_path: &module_path,
            module,
            struct_,
        };
        template.write_into(&mut File::create(
            base_path.join(format!("struct.{}.html", struct_.name)),
        )?)?;
    }

    for function in &module.functions {
        let module_path =
            module_path.extend(function.name.to_string(), "#", ItemKind::Function, false);
        let template = FunctionTemplate {
            base,
            title: &function.name.to_string(),
            module_path: &module_path,
            module,
            function,
        };
        template.write_into(&mut File::create(
            base_path.join(format!("fn.{}.html", function.name)),
        )?)?;
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum ItemKind {
    Module,
    Constant,
    GlobalVariable,
    Struct,
    Function,
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
    constant: &'a Constant,
}

#[derive(Template)]
#[template(path = "global_variable.html")]
struct GlobalVariableTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    module: &'a Module,
    var: &'a GlobalVariable,
}

#[derive(Template)]
#[template(path = "struct.html")]
struct StructTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    module: &'a Module,
    struct_: &'a Struct,
}

#[derive(Template)]
#[template(path = "function.html")]
struct FunctionTemplate<'a> {
    base: &'a Base<'a>,
    title: &'a str,
    module_path: &'a ModulePath,
    module: &'a Module,
    function: &'a Function,
}

#[derive(Template)]
#[template(path = "render_type.html")]
struct RenderTypeTemplate<'a> {
    ty: &'a Type,
    module_path_level: usize,
}

impl RenderTypeTemplate<'_> {
    fn render_rec(&self, ty: &Type) -> String {
        RenderTypeTemplate {
            ty,
            module_path_level: self.module_path_level,
        }
        .to_string()
    }
}

fn render_type(ty: &Type, module_path_level: &usize) -> String {
    RenderTypeTemplate {
        ty,
        module_path_level: *module_path_level,
    }
    .to_string()
}

struct Base<'a> {
    doc: &'a WeslDocs,
    build_as_latest: bool,
}

fn module_path_class(kind: &ItemKind, last: &bool) -> &'static str {
    if !*last {
        return "";
    }

    match kind {
        ItemKind::Module => "module",
        ItemKind::Constant => "const",
        ItemKind::GlobalVariable => "var",
        ItemKind::Struct => "struct",
        ItemKind::Function => "fn",
    }
}

fn display_array_size(size: &Option<u32>) -> String {
    match size {
        Some(size) => size.to_string(),
        None => "?".to_string(),
    }
}

// Copy-pasted and adapted from: naga-0.14.1

fn display_binding(binding: Option<&Binding>) -> String {
    let Some(binding) = binding else {
        return "".to_string();
    };

    match binding {
        Binding::BuiltIn(builtin) => {
            let builtin = builtin_str(builtin).unwrap();
            format!("@builtin({}) ", builtin)
        }
        Binding::Location {
            location,
            second_blend_source,
            interpolation,
            sampling,
        } => {
            let mut res = String::new();

            res += &format!("@location({}) ", location);
            if *second_blend_source {
                res += "@second_blend_source ";
            }

            if sampling.is_some() && *sampling != Some(Sampling::Center) {
                res += &format!(
                    "@interpolate({}, {}) ",
                    interpolation_str(interpolation.unwrap_or(Interpolation::Perspective)),
                    sampling_str(sampling.unwrap_or(Sampling::Center))
                );
            } else if interpolation.is_some() && *interpolation != Some(Interpolation::Perspective)
            {
                res += &format!(
                    "@interpolate({}) ",
                    interpolation_str(interpolation.unwrap_or(Interpolation::Perspective))
                );
            }

            res
        }
    }
}

fn builtin_str(built_in: &BuiltIn) -> Result<&'static str, Box<dyn std::error::Error>> {
    Ok(match built_in {
        BuiltIn::Position { .. } => "position",
        BuiltIn::ViewIndex => "view_index",
        BuiltIn::InstanceIndex => "instance_index",
        BuiltIn::VertexIndex => "vertex_index",
        BuiltIn::FragDepth => "frag_depth",
        BuiltIn::FrontFacing => "front_facing",
        BuiltIn::PrimitiveIndex => "primitive_index",
        BuiltIn::SampleIndex => "sample_index",
        BuiltIn::SampleMask => "sample_mask",
        BuiltIn::GlobalInvocationId => "global_invocation_id",
        BuiltIn::LocalInvocationId => "local_invocation_id",
        BuiltIn::LocalInvocationIndex => "local_invocation_index",
        BuiltIn::WorkGroupId => "workgroup_id",
        BuiltIn::NumWorkGroups => "num_workgroups",
        BuiltIn::NumSubgroups => "num_subgroups",
        BuiltIn::SubgroupId => "subgroup_id",
        BuiltIn::SubgroupSize => "subgroup_size",
        BuiltIn::SubgroupInvocationId => "subgroup_invocation_id",
        BuiltIn::BaseInstance
        | BuiltIn::BaseVertex
        | BuiltIn::ClipDistance
        | BuiltIn::CullDistance
        | BuiltIn::PointSize
        | BuiltIn::PointCoord
        | BuiltIn::WorkGroupSize
        | BuiltIn::DrawID => return Err(format!("unsupported built-in: {:?}", built_in).into()),
    })
}

const fn interpolation_str(interpolation: Interpolation) -> &'static str {
    match interpolation {
        Interpolation::Perspective => "perspective",
        Interpolation::Linear => "linear",
        Interpolation::Flat => "flat",
    }
}

const fn sampling_str(sampling: Sampling) -> &'static str {
    match sampling {
        Sampling::Center => "",
        Sampling::Centroid => "centroid",
        Sampling::Sample => "sample",
        Sampling::First => "first",
        Sampling::Either => "either",
    }
}
