mod cargo;
mod resolver;
mod wesl_toml;

use self::{
    cargo::{CargoMetadata, CargoPackage},
    resolver::DocsResolver,
    wesl_toml::{DependenciesAuto, WeslToml, WeslTomlDependency},
};
use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
};
use wesl::{CompileOptions, Feature, Features, ManglerKind, ModulePath, Wesl, syntax::PathOrigin};
use wesldoc_ast::Version;
use wesldoc_compiler::{MissingDocumentation, WeslModule, WeslPackage};

pub use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The path to the package to generate docs for.
    package: PathBuf,

    /// The path to the output directory.
    #[arg(short, long, default_value = "target/wesldoc")]
    output: PathBuf,

    /// The missing documentation behavior.
    #[arg(long, value_enum, default_value = "allow")]
    missing_docs: MissingDocsArg,

    /// Whether to print documentation statistics after compilation.
    #[arg(long, default_value = "false")]
    statistics: bool,
}

impl Args {
    pub fn run(self) -> Result<()> {
        // Check Cargo.toml exist
        if !self.package.join("Cargo.toml").is_file() {
            bail!("Cargo.toml not found");
        }

        // Load wesl.toml
        let wesl_toml = load_wesl_toml(self.package.join("wesl.toml"))?;

        // Resolve cargo dependencies
        let (cargo_metadata, cargo_root_package) = CargoMetadata::resolve(&self.package)?;

        // Create package and resolver
        let package = Package::from_cargo_package(&cargo_root_package, None)?;
        let resolver = match wesl_toml.package.dependencies {
            Some(DependenciesAuto::Auto) => {
                DocsResolver::new_auto(&package, cargo_metadata, cargo_root_package)
            }
            None => {
                let dependencies = wesl_toml
                    .dependencies
                    .iter()
                    .map(|(dep_key, dep)| {
                        Package::new_dependency(
                            &cargo_root_package,
                            dep_key,
                            Some(dep),
                            &cargo_metadata,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;
                DocsResolver::new_explicit(&package, dependencies)
            }
        };

        // Compile to wesl
        let wesl_package = compile_package(package, resolver)?;

        // Compile to docs
        let (docs, compile_stats) = wesldoc_compiler::compile(
            &wesl_package,
            &wesldoc_compiler::CompileOptions {
                missing_documentation: self.missing_docs.into(),
            },
        )
        .with_context(|| format!("failed to compile package '{}'", wesl_package.root.name))?;
        if self.statistics {
            println!(
                "Documentation Coverage: {:.2}%",
                compile_stats.documented_percentage()
            );
        }

        // Generate docs
        wesldoc_generator::generate(&docs, &self.output)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum MissingDocsArg {
    /// Allow missing documentation.
    Allow,
    /// Warn on missing documentation.
    Warn,
    /// Error on missing documentation.
    Deny,
}

impl From<MissingDocsArg> for MissingDocumentation {
    fn from(arg: MissingDocsArg) -> Self {
        match arg {
            MissingDocsArg::Allow => MissingDocumentation::Allow,
            MissingDocsArg::Warn => MissingDocumentation::Warn,
            MissingDocsArg::Deny => MissingDocumentation::Deny,
        }
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

            let compile_result = wesl.compile(&ModulePath {
                origin: PathOrigin::Absolute,
                components: path
                    .strip_prefix(root)?
                    .components()
                    .map(|part| match part {
                        Component::Normal(name) => {
                            let name = name.to_string_lossy().to_string();
                            let name = name
                                .strip_suffix(".wesl")
                                .or_else(|| name.strip_suffix(".wgsl"))
                                .map(|s| s.to_string())
                                .unwrap_or(name);
                            Ok(name)
                        }
                        _ => bail!("unexpected path component"),
                    })
                    .collect::<Result<_>>()?,
            })?;
            let root_file_imports = wesl.resolver().take_root_file_imports();
            sub.compiled = Some((root_file_imports, compile_result));
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
    version: Version,
    root: PathBuf,
}

impl Package {
    fn from_cargo_package(
        cargo_package: &CargoPackage,
        local_name: Option<String>,
    ) -> Result<Self> {
        let wesl_toml = load_wesl_toml(cargo_package.crate_path().join("wesl.toml"))?;

        let local_name = local_name.unwrap_or_else(|| cargo_package.name());
        let package_name = cargo_package.name();
        let version = cargo_package.version();
        let root = cargo_package.crate_path().join(&wesl_toml.package.root);

        Ok(Self {
            local_name,
            package_name,
            version,
            root,
        })
    }

    fn new_dependency(
        this_cargo_package: &CargoPackage,

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
            let dep_path = this_cargo_package.crate_path().join(dep_path);
            let dep_wesl_toml = load_wesl_toml(dep_path.join("wesl.toml"))?;

            return Ok(Package {
                local_name: dependency_key.clone(),
                package_name: dep_name.clone(),
                root: dep_path.join(&dep_wesl_toml.package.root),
                version: Version::new(0, 0, 0), // TODO: path dependencies don't have versions
            });
        }

        let dep_pkg_id = this_cargo_package
            .dep(dep_name)
            .with_context(|| format!("dependency '{dep_name}' not found in Cargo.toml"))?;
        let dep_pkg = cargo_metadata
            .package(dep_pkg_id)
            .context("invalid dependency")?;
        Package::from_cargo_package(dep_pkg, Some(dependency_key))
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

fn load_wesl_toml(path: impl AsRef<Path>) -> Result<WeslToml> {
    let path = path.as_ref();
    let wesl_toml = if path.is_file() {
        toml::from_slice::<WeslToml>(&fs::read(path)?)?
    } else {
        WeslToml::default()
    };
    wesl_toml.validate()?;
    Ok(wesl_toml)
}
