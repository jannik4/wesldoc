mod all_items;
mod context;
mod index;
mod render;
mod static_files;

use crate::{context::Context, render::*};
use askama::Template;
use serde_json::Value;
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs::{self, File},
    path::Path,
};
use wesldoc_ast::{ItemKind, Version, WeslDocs};

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
        &Context::new(build_as_latest, doc),
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

fn gen_module(ctx: &Context, base_path_docs: &Path, base_path_src: &Path) -> Result<()> {
    if let Some(source) = &ctx.module.source {
        let template = SourceTemplate {
            ctx,
            title: &ctx.module.name,
            source,
            is_source_view: (),
        };
        let mut path = base_path_src.to_path_buf();
        path.set_extension("html");
        template.write_into(&mut File::create(path)?)?;
    }

    let template = OverviewTemplate {
        ctx,
        title: &ctx.module.name,
    };
    template.write_into(&mut File::create(base_path_docs.join("index.html"))?)?;

    for module in &ctx.module.modules {
        let ctx = ctx.with_submodule(module);

        let base_path_docs = base_path_docs.join(&module.name);
        fs::create_dir(&base_path_docs)?;

        let base_path_src = base_path_src.join(&module.name);
        fs::create_dir(&base_path_src)?;

        gen_module(&ctx, &base_path_docs, &base_path_src)?;
    }

    for (name, item) in &ctx.module.constants {
        let ctx = ctx.with_item(name.to_string(), ItemKind::Constant);
        let template = ConstantTemplate {
            ctx: &ctx,
            title: &name.to_string(),
            constants: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("const.{}.html", name)),
        )?)?;
    }

    for (name, item) in &ctx.module.overrides {
        let ctx = ctx.with_item(name.to_string(), ItemKind::Override);
        let template = OverrideTemplate {
            ctx: &ctx,
            title: &name.to_string(),
            overrides: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("override.{}.html", name)),
        )?)?;
    }

    for (name, item) in &ctx.module.global_variables {
        let ctx = ctx.with_item(name.to_string(), ItemKind::GlobalVariable);
        let template = GlobalVariableTemplate {
            ctx: &ctx,
            title: &name.to_string(),
            variables: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("var.{}.html", name)),
        )?)?;
    }

    for (name, item) in &ctx.module.structs {
        let ctx = ctx.with_item(name.to_string(), ItemKind::Struct);
        let template = StructTemplate {
            ctx: &ctx,
            title: &name.to_string(),
            structs: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("struct.{}.html", name)),
        )?)?;
    }

    for (name, item) in &ctx.module.functions {
        let ctx = ctx.with_item(name.to_string(), ItemKind::Function);
        let template = FunctionTemplate {
            ctx: &ctx,
            title: &name.to_string(),
            functions: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("fn.{}.html", name)),
        )?)?;
    }

    for (name, item) in &ctx.module.type_aliases {
        let ctx = ctx.with_item(name.to_string(), ItemKind::TypeAlias);
        let template = TypeAliasTemplate {
            ctx: &ctx,
            title: &name.to_string(),
            type_aliases: &item.instances,
        };
        template.write_into(&mut File::create(
            base_path_docs.join(format!("alias.{}.html", name)),
        )?)?;
    }

    Ok(())
}
