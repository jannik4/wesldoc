mod resolver;
mod wesl_toml;

use crate::wesl_toml::WeslTomlPackageManager;

use self::{resolver::DocsResolver, wesl_toml::WeslToml};
use clap::Parser;
use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
};
use wesl::{CompileOptions, Feature, Features, ManglerKind, ModulePath, Wesl, syntax::PathOrigin};
use wesldoc_ast::Version;
use wesldoc_compiler::{WeslModule, WeslPackage};

pub type Error = Box<dyn std::error::Error>;
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The path to the package to generate docs for.
    package: PathBuf,

    /// The path to the output directory.
    #[arg(short, long, default_value = "target/wesldoc")]
    output: PathBuf,
}

impl Args {
    pub fn run(self) -> Result<()> {
        // Check Cargo.toml/wesl.toml exist
        if !self.package.join("Cargo.toml").is_file() {
            return Err("Cargo.toml not found".into());
        }
        if !self.package.join("wesl.toml").is_file() {
            return Err("wesl.toml not found".into());
        }

        // Parse wesl.toml and validate
        let wesl_toml = toml::from_slice::<WeslToml>(&fs::read(self.package.join("wesl.toml"))?)?;
        validate_wesl_toml(&wesl_toml)?;

        // Resolve cargo dependencies
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(self.package.join("Cargo.toml"))
            .exec()?;
        let metadata_root_package = metadata.root_package().ok_or("no root package")?;
        let resolved_deps = {
            let resolve = metadata.resolve.as_ref().ok_or("no resolve")?;
            let root_node = resolve
                .nodes
                .iter()
                .find(|node| node.id == metadata_root_package.id)
                .ok_or("no root node")?;
            root_node
                .deps
                .iter()
                .map(|dep| (dep.name.clone(), dep.pkg.clone()))
                .collect::<HashMap<_, _>>()
        };

        // Get package and dependencies
        let package = Package::new(
            metadata_root_package.name.to_string(),
            &self.package,
            metadata_root_package,
            &wesl_toml,
        );
        let dependencies = wesl_toml
            .dependencies
            .iter()
            .map(|(dep_key, dep)| {
                if dep.path.is_some() {
                    // TODO: this needs to get the version of the path dependency using a separate
                    // cargo metadata call? Or completely ignore versions for path dependencies?
                    return Err("path dependencies are not supported yet".into());
                }

                let dep_name = dep.package.as_ref().unwrap_or(dep_key);
                let dep_pkg_id = resolved_deps
                    .get(dep_name)
                    .ok_or(format!("dependency '{dep_name}' not found in Cargo.toml"))?;
                let metadata_package = metadata
                    .packages
                    .iter()
                    .find(|pkg| &pkg.id == dep_pkg_id)
                    .unwrap();
                let crate_path = metadata_package
                    .manifest_path
                    .parent()
                    .unwrap()
                    .to_path_buf()
                    .into_std_path_buf();

                let dep_wesl_toml =
                    toml::from_slice::<WeslToml>(&fs::read(crate_path.join("wesl.toml"))?)?;
                validate_wesl_toml(&dep_wesl_toml)?;

                Ok(Package::new(
                    dep_key.clone(),
                    &crate_path,
                    metadata_package,
                    &dep_wesl_toml,
                ))
            })
            .collect::<Result<_>>()?;

        // Compile to wesl
        let wesl_package = compile_package(package, dependencies)?;

        // Compile to docs
        let docs = wesldoc_compiler::compile(&wesl_package)?;

        // Generate docs
        wesldoc_generator::generate(&docs, Path::new("target/wesldoc"))?;

        Ok(())
    }
}

fn validate_wesl_toml(toml: &WeslToml) -> Result<()> {
    if toml.package.edition != "unstable_2025" {
        return Err("only edition 'unstable_2025' is supported".into());
    }

    match toml.package.package_manager {
        Some(WeslTomlPackageManager::Cargo) | None => (),
        Some(WeslTomlPackageManager::Npm) => {
            return Err("npm package manager is not supported yet".into());
        }
    }

    for dep in toml.dependencies.values() {
        if dep.package.is_some() && dep.path.is_some() {
            return Err("dependency cannot have both 'package' and 'path'".into());
        }
    }

    Ok(())
}

fn compile_package(package: Package, dependencies: Vec<Package>) -> Result<WeslPackage> {
    let wesl = {
        let resolver = DocsResolver::new(&package, &dependencies);
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
                mangle_root: false,
                keep: None,
                keep_root: true,
                features: Features {
                    default: Feature::Keep,
                    flags: HashMap::default(),
                },
            });
        wesl
    };

    Ok(WeslPackage {
        version: package.version,
        dependencies: dependencies
            .iter()
            .map(|dep| (dep.local_name.clone(), dep.version.clone()))
            .collect(),
        root: WeslModule {
            name: package.package_name,
            compiled: None,
            submodules: compile_submodules(&wesl, &package.root, &package.root)?,
        },
    })
}

fn compile_submodules(
    wesl: &Wesl<DocsResolver>,
    dir: &Path,
    root: &Path,
) -> Result<Vec<WeslModule>> {
    let mut submodules = HashMap::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        let name = name_from_path(&path)?;

        if path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext == "wesl" || ext == "wgsl")
        {
            let sub = submodules
                .entry(name)
                .or_insert_with_key(|name| WeslModule {
                    name: name.clone(),
                    compiled: None,
                    submodules: Vec::new(),
                });
            sub.compiled = Some(
                wesl.compile(&ModulePath {
                    origin: PathOrigin::Absolute,
                    components: path
                        .strip_prefix(root)?
                        .components()
                        .map(|part| match part {
                            Component::Normal(name) => Ok(name.to_string_lossy().to_string()),
                            _ => Err("unexpected path component".into()),
                        })
                        .collect::<Result<_>>()?,
                })?,
            );
        } else if path.is_dir() {
            let sub = submodules
                .entry(name)
                .or_insert_with_key(|name| WeslModule {
                    name: name.clone(),
                    compiled: None,
                    submodules: Vec::new(),
                });
            sub.submodules = compile_submodules(wesl, &path, root)?;
        }
    }

    Ok(submodules.into_values().collect())
}

#[derive(Debug, Clone)]
struct Package {
    local_name: String,
    package_name: String,
    root: PathBuf,
    version: Version,
}

impl Package {
    fn new(
        local_name: String,
        base_path: impl AsRef<Path>,
        metadata_package: &cargo_metadata::Package,
        wesl_toml: &WeslToml,
    ) -> Self {
        Self {
            local_name,
            package_name: metadata_package.name.to_string(),
            root: base_path.as_ref().join(&wesl_toml.package.root),
            version: metadata_package.version.clone(),
        }
    }
}

fn name_from_path(path: &Path) -> Result<String> {
    let path = path.canonicalize()?;
    Ok(path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .replace('-', "_"))
}
