mod package;
mod resolver;

use self::{
    package::{Module, Package},
    resolver::DocsResolver,
};
use std::{collections::HashMap, path::Path};
use wesl::{CompileOptions, Feature, Features, ManglerKind, Wesl};
use wesldoc_ast::Version;
use wesldoc_compiler::{WeslModule, WeslPackage};

pub type Error = Box<dyn std::error::Error>;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub fn make(name: &str, dependencies: &[&str]) -> Result<()> {
    // Read package
    let package = Package::read_from_dir(
        name,
        Version::new(0, 0, 0),
        format!("./example_packages/{}", name),
    )?;

    // Read dependencies
    let dependencies = dependencies
        .iter()
        .map(|name| {
            Ok((
                name.to_string(),
                Package::read_from_dir(
                    *name,
                    Version::new(0, 0, 0),
                    format!("./example_packages/{}", name),
                )?,
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    // Compile to wesl
    let wesl_package = compile_package(Version::new(0, 0, 0), name, &package, dependencies)?;

    // Compile to docs
    let docs = wesldoc_compiler::compile(&wesl_package)?;

    // Generate docs
    wesldoc_generator::generate(&docs, Path::new("target/wesldoc"))?;

    Ok(())
}

fn compile_package(
    version: Version,
    name: &str,
    package: &Package,
    dependencies: HashMap<String, Package>,
) -> Result<WeslPackage> {
    let deps = dependencies
        .iter()
        .map(|(name, package)| (name.clone(), package.version.clone()))
        .collect();

    let base_path = format!("./example_packages/{}", name);
    let wesl = {
        let resolver = DocsResolver::new(dependencies, &base_path);
        let mut wesl = Wesl::new_barebones().set_custom_resolver(resolver);
        wesl.set_mangler(ManglerKind::Escape)
            .use_sourcemap(true)
            .set_options(CompileOptions {
                imports: true,
                condcomp: true,
                generics: false,
                strip: false,
                lower: false,
                validate: false,
                lazy: true,
                keep: None,
                features: Features {
                    default: Feature::Keep,
                    flags: HashMap::default(),
                },
            });
        wesl
    };

    Ok(WeslPackage {
        version,
        dependencies: deps,
        root: compile_module(&package.root, &wesl, base_path.as_ref())?,
    })
}

fn compile_module(
    module: &Module,
    wesl: &Wesl<DocsResolver>,
    base_path: &Path,
) -> Result<WeslModule> {
    Ok(WeslModule {
        name: module.name.clone(),
        compiled: match &module.file {
            Some(file) => Some(wesl.compile(file.path.strip_prefix(base_path)?)?),
            None => None,
        },
        submodules: module
            .submodules
            .values()
            .map(|m| compile_module(m, wesl, base_path))
            .collect::<Result<_>>()?,
    })
}
