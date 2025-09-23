mod resolver;
mod wesl_toml;

use self::{
    resolver::DocsResolver,
    wesl_toml::{DependenciesAuto, WeslToml, WeslTomlDependency},
};
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
        wesl_toml.validate()?;

        // Resolve cargo dependencies
        let cargo_metadata = CargoMetadata::resolve(&self.package)?;

        // Create package and resolver
        let package = Package::new(
            cargo_metadata.root_package.name.to_string(),
            &self.package,
            &cargo_metadata.root_package,
            &wesl_toml,
        );
        let resolver = match wesl_toml.package.dependencies {
            Some(DependenciesAuto::Auto) => DocsResolver::new_auto(&package, cargo_metadata),
            None => {
                let dependencies = wesl_toml
                    .dependencies
                    .iter()
                    .map(|(dep_key, dep)| {
                        Package::new_dependency(dep_key, Some(dep), &cargo_metadata)
                    })
                    .collect::<Result<Vec<_>>>()?;
                DocsResolver::new_explicit(&package, dependencies)
            }
        };

        // Compile to wesl
        let wesl_package = compile_package(package, resolver)?;

        // Compile to docs
        let docs = wesldoc_compiler::compile(&wesl_package)?;

        // Generate docs
        wesldoc_generator::generate(&docs, &self.output)?;

        Ok(())
    }
}

fn compile_package(package: Package, resolver: DocsResolver) -> Result<WeslPackage> {
    let wesl = {
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

    // Compile root and submodules
    let root = WeslModule {
        name: package.package_name,
        compiled: None,
        submodules: compile_submodules(&wesl, &package.root, &package.root)?,
    };

    // Get resolved dependencies
    let dependencies = wesl
        .resolver()
        .resolved_dependencies()
        .into_iter()
        .map(|dep| (dep.local_name, (dep.package_name, dep.version)))
        .collect();

    Ok(WeslPackage {
        version: package.version,
        dependencies,
        root,
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

struct CargoMetadata {
    base_path: PathBuf,
    metadata: cargo_metadata::Metadata,
    root_package: cargo_metadata::Package,
    resolved_dependencies: HashMap<String, cargo_metadata::PackageId>,
}

impl CargoMetadata {
    fn resolve(base_path: impl Into<PathBuf>) -> Result<Self> {
        let base_path = base_path.into();
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(base_path.join("Cargo.toml"))
            .exec()?;
        let root_package = metadata.root_package().ok_or("no root package")?.clone();
        let resolve = metadata.resolve.as_ref().ok_or("no resolve")?;
        let root_node = resolve
            .nodes
            .iter()
            .find(|node| node.id == root_package.id)
            .ok_or("no root node")?;
        let resolved_dependencies = root_node
            .deps
            .iter()
            .map(|dep| (dep.name.clone(), dep.pkg.clone()))
            .collect::<HashMap<_, _>>();

        Ok(Self {
            base_path,
            metadata,
            root_package,
            resolved_dependencies,
        })
    }
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

    fn new_dependency(
        dependency_key: impl Into<String>,
        dependency: Option<&WeslTomlDependency>,
        cargo_metadata: &CargoMetadata,
    ) -> Result<Self> {
        let dependency_key = dependency_key.into();
        let dep_name = dependency
            .and_then(|d| d.package.as_ref())
            .unwrap_or(&dependency_key);

        // Handle path dependencies
        if let Some(dep_path) = dependency.and_then(|d| d.path.as_ref()) {
            let dep_path = cargo_metadata.base_path.join(dep_path);
            let dep_wesl_toml =
                toml::from_slice::<WeslToml>(&fs::read(dep_path.join("wesl.toml"))?)?;
            dep_wesl_toml.validate()?;

            return Ok(Package {
                local_name: dependency_key.clone(),
                package_name: dep_name.clone(),
                root: dep_path.join(&dep_wesl_toml.package.root),
                version: Version::new(0, 0, 0), // TODO: path dependencies don't have versions
            });
        }

        let dep_pkg_id = cargo_metadata
            .resolved_dependencies
            .get(dep_name)
            .ok_or(format!("dependency '{dep_name}' not found in Cargo.toml"))?;
        let metadata_package = cargo_metadata
            .metadata
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

        let dep_wesl_toml = toml::from_slice::<WeslToml>(&fs::read(crate_path.join("wesl.toml"))?)?;
        dep_wesl_toml.validate()?;

        Ok(Package::new(
            dependency_key,
            &crate_path,
            metadata_package,
            &dep_wesl_toml,
        ))
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
