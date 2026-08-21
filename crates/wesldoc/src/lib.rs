mod cargo;
// TODO: Remove and delete: mod resolver;
mod preprocess;
mod wesl_toml;

use self::{
    cargo::{CargoMetadata, CargoPackage},
    wesl_toml::{DependenciesAuto, WeslToml, WeslTomlDependency},
};
use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use std::{
    collections::{HashMap, hash_map::Entry},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use wesldoc_ast::{Conditional, DefinitionPath, ItemKind, Version};
use wesldoc_compiler::{
    MissingDocumentation, ResolvedItem, Resolver, ResolverResult, WeslModule,
    build_conditional::conditional_from_attributes,
};
use wgsl_parse::{SyntaxNode, syntax};

pub use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The path to the package to generate docs for.
    package: PathBuf,

    /// Don't build documentation for dependencies.
    #[arg(long, default_value = "false")]
    no_deps: bool,

    /// The maximum depth of dependencies to document. If not specified, all dependencies will be
    /// documented. `--no-deps` will override this option and no dependencies will be documented.
    #[arg(long)]
    max_dependency_depth: Option<usize>,

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

        // Resolve cargo dependencies
        let cargo_metadata = Arc::new(CargoMetadata::resolve(&self.package)?);

        // Cache
        let mut cache = Cache::new(Arc::clone(&cargo_metadata));

        // Doc packages
        let max_depth = match self.no_deps {
            true => 0,
            false => self.max_dependency_depth.unwrap_or(usize::MAX),
        };
        let mut wesl_package_found = false;
        for cargo_package in cargo_metadata.iter_packages(max_depth) {
            // Package from cargo package and check if it is a wesl package
            let package = Package::from_cargo_package(cargo_package)?;
            if !is_wesl_package(&package)? {
                continue;
            }
            wesl_package_found = true;

            println!(
                "Documenting package: {} v{}",
                package.package_name, package.version
            );

            // ...
            let wesl_module = cache
                .get(package.id.clone())?
                .context("expected wesl package")?;
            let dependencies = match package.wesl_toml.package.dependencies {
                Some(DependenciesAuto::Auto) => Dependencies::Auto {
                    dependencies: HashMap::new(),
                },
                None => Dependencies::Explicit {
                    dependencies: package
                        .wesl_toml
                        .dependencies
                        .iter()
                        .map(|(dep_key, dep)| {
                            Ok((
                                dep_key.clone(),
                                Package::new_dependency(
                                    cargo_package,
                                    dep_key,
                                    Some(dep),
                                    &cargo_metadata,
                                )?,
                            ))
                        })
                        .collect::<Result<_>>()?,
                },
            };
            let resolver = CompilePackageResolver::new(&mut cache, package, dependencies);

            // Compile to docs
            let (docs, compile_stats) = wesldoc_compiler::compile(
                resolver,
                &wesl_module,
                &wesldoc_compiler::CompileOptions {
                    missing_documentation: self.missing_docs.into(),
                },
            )
            .with_context(|| format!("failed to compile package '{}'", wesl_module.name))?;
            if self.statistics {
                println!(
                    "Documentation Coverage: {:.2}%",
                    compile_stats.documented_percentage()
                );
            }

            // Generate docs
            wesldoc_generator::generate(&docs, &self.output)?;
        }

        if !wesl_package_found {
            bail!("No wesl packages found in the specified path");
        }

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

// Only count as a wesl package if it has a wesl.toml file or at least one .wesl file
fn is_wesl_package(package: &Package) -> Result<bool> {
    if package.has_wesl_toml_file {
        return Ok(true);
    }

    if !package.root.is_dir() {
        return Ok(false);
    }
    let mut dirs = vec![package.root.clone()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "wesl") {
                return Ok(true);
            } else if path.is_dir() {
                dirs.push(path);
            }
        }
    }

    Ok(false)
}

#[derive(Debug, Clone)]
struct Package {
    package_name: String,
    version: Version,
    wesl_toml: WeslToml,
    has_wesl_toml_file: bool,
    root: PathBuf,

    id: PackageId,
}

impl Package {
    fn from_cargo_package(cargo_package: &CargoPackage) -> Result<Self> {
        let (wesl_toml, has_wesl_toml_file) =
            load_wesl_toml(cargo_package.crate_path().join("wesl.toml"))?;

        let package_name = cargo_package.name();
        let version = cargo_package.version();
        let root = cargo_package.crate_path().join(&wesl_toml.package.root);

        Ok(Self {
            package_name,
            version,
            wesl_toml,
            has_wesl_toml_file,
            root,

            id: cargo_package.id().clone(),
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
            let dep_path = this_cargo_package
                .crate_path()
                .join(dep_path)
                .canonicalize()?;
            let (dep_wesl_toml, dep_has_wesl_toml_file) =
                load_wesl_toml(dep_path.join("wesl.toml"))?;

            return Ok(Package {
                package_name: dep_name.clone(),
                root: dep_path.join(&dep_wesl_toml.package.root),
                wesl_toml: dep_wesl_toml,
                has_wesl_toml_file: dep_has_wesl_toml_file,
                version: Version::new(0, 0, 0), // TODO: path dependencies don't have versions

                id: PackageId::Path(dep_path),
            });
        }

        let dep_pkg_id = this_cargo_package
            .dep(dep_name)
            .with_context(|| format!("dependency '{dep_name}' not found in Cargo.toml"))?;
        let dep_pkg = cargo_metadata
            .package(dep_pkg_id)
            .context("invalid dependency")?;
        Package::from_cargo_package(dep_pkg)
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

fn load_wesl_toml(path: impl AsRef<Path>) -> Result<(WeslToml, bool)> {
    let path = path.as_ref();
    let (wesl_toml, has_wesl_toml_file) = if path.is_file() {
        (toml::from_slice::<WeslToml>(&fs::read(path)?)?, true)
    } else {
        (WeslToml::default(), false)
    };
    wesl_toml.validate()?;
    Ok((wesl_toml, has_wesl_toml_file))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PackageId {
    Cargo(cargo_metadata::PackageId),
    Path(PathBuf),
}

struct Cache {
    packages: HashMap<PackageId, Arc<WeslModule>>,
    cargo_metadata: Arc<CargoMetadata>,
}

impl Cache {
    fn new(cargo_metadata: Arc<CargoMetadata>) -> Self {
        Self {
            packages: HashMap::new(),
            cargo_metadata,
        }
    }

    fn get(&mut self, package_id: PackageId) -> Result<Option<Arc<WeslModule>>> {
        match self.packages.entry(package_id) {
            Entry::Occupied(pkg) => Ok(Some(Arc::clone(pkg.get()))),
            Entry::Vacant(entry) => {
                let package = match entry.key() {
                    PackageId::Cargo(package_id) => {
                        let cargo_package = match self.cargo_metadata.package(package_id) {
                            Some(pkg) => pkg,
                            None => return Ok(None),
                        };
                        Package::from_cargo_package(cargo_package)?
                    }
                    PackageId::Path(_path_buf) => todo!(),
                };
                let wesl_module = Arc::new(compile_package(package)?);
                entry.insert(Arc::clone(&wesl_module));
                Ok(Some(wesl_module))
            }
        }
    }
}

struct CompilePackageResolver<'a> {
    cache: &'a mut Cache,
    package: Package,
    dependencies: Dependencies,
}

impl<'a> CompilePackageResolver<'a> {
    fn new(cache: &'a mut Cache, package: Package, dependencies: Dependencies) -> Self {
        Self {
            cache,
            package,
            dependencies,
        }
    }

    fn get_syntax(
        &mut self,
        package_id: PackageId,
        path: &[String],
    ) -> Option<syntax::TranslationUnit> {
        let wesl_package = self.cache.get(package_id).ok().flatten()?;

        // Navigate to the module specified by the path
        let mut module = &*wesl_package;
        for component in path {
            let submodule = module.submodules.iter().find(|m| m.name == *component)?;
            module = submodule;
        }
        let (syntax, _) = module.code.as_ref()?;

        Some(syntax.clone()) // TODO: do not clone!
    }

    // TODO: detect infinite loops!!! (e.g. keep track of visited (package.id, path), break on cycle)
    fn resolve_in(
        &mut self,
        package: Package,
        include_imports: bool,
        path: &[String],
        name: &str,
        results: &mut Vec<ResolvedItem>,
        condition: Conditional,
    ) {
        let Some(syntax) = self.get_syntax(package.id.clone(), path) else {
            return;
        };

        // Lookup name in imports/exports
        for import in &syntax.imports {
            let is_export = import.attributes.iter().any(|attr| attr.is_publish());
            if !include_imports && !is_export {
                continue;
            }

            self.resolve_import(
                &package,
                import,
                path,
                &[name.to_string()],
                results,
                condition.clone(),
            );
        }

        // Lookup name in global declarations
        for decl in &syntax.global_declarations {
            let Some((decl_name, decl_kind)) = decl_info(decl) else {
                continue;
            };
            if *decl_name.name() != name {
                continue;
            }
            let decl_condition =
                conditional_from_attributes(decl.attributes()).unwrap_or(Conditional::True);

            results.push(ResolvedItem {
                kind: decl_kind,
                def_path: if package.id == self.package.id {
                    DefinitionPath::Absolute(path.to_vec())
                } else {
                    DefinitionPath::Package(
                        package.package_name.clone(),
                        package.version.clone(),
                        path.to_vec(),
                    )
                },
                conditional: Some(Conditional::And(
                    Box::new(condition.clone()),
                    Box::new(decl_condition),
                )),
            });
        }
    }

    fn resolve_import(
        &mut self,
        package: &Package,
        import: &syntax::ImportStatement,
        path: &[String],
        item_path_and_name: &[String],
        results: &mut Vec<ResolvedItem>,
        condition: Conditional,
    ) {
        let Some(import_path) = &import.path else {
            // TODO: Handle this
            return;
        };
        let import_condition =
            conditional_from_attributes(import.attributes()).unwrap_or(Conditional::True);

        let mut to_resolve = vec![(import_path.clone(), &import.content)];
        while let Some((mut import_path, content)) = to_resolve.pop() {
            match content {
                syntax::ImportContent::Item(import_item) => {
                    // Check name
                    match &import_item.rename {
                        Some(rename) => {
                            if *rename.name() != item_path_and_name[0] {
                                continue;
                            }
                        }
                        None => {
                            if *import_item.ident.name() != item_path_and_name[0] {
                                continue;
                            }
                        }
                    }

                    import_path
                        .components
                        .extend_from_slice(&item_path_and_name[..item_path_and_name.len() - 1]);
                    let name = item_path_and_name.last().unwrap();

                    // And condition with import's conditional
                    let condition = Conditional::And(
                        Box::new(condition.clone()),
                        Box::new(import_condition.clone()),
                    );

                    // Resolve
                    match import_path.origin {
                        syntax::PathOrigin::Absolute => {
                            let path = &import_path.components;
                            self.resolve_in(
                                package.clone(), // TODO: do not clone!
                                false,
                                path,
                                name,
                                results,
                                condition,
                            );
                        }
                        syntax::PathOrigin::Relative(n) => {
                            let to_keep = path.len().saturating_sub(n);
                            let path = path
                                .iter()
                                .take(to_keep)
                                .chain(&import_path.components)
                                .cloned()
                                .collect::<Vec<_>>();

                            self.resolve_in(
                                package.clone(), // TODO: do not clone!
                                false,
                                &path,
                                name,
                                results,
                                condition,
                            );
                        }
                        syntax::PathOrigin::Package(package_name) => {
                            let package = match &mut self.dependencies {
                                Dependencies::Explicit { dependencies } => {
                                    match dependencies.get(&package_name) {
                                        Some(pkg) => pkg.clone(),
                                        None => {
                                            println!(
                                                "Warning: dependency '{}' not found",
                                                package_name
                                            );
                                            continue;
                                        }
                                    }
                                }
                                Dependencies::Auto { dependencies } => {
                                    match dependencies.entry(package_name.clone()) {
                                        Entry::Occupied(entry) => entry.get().clone(),
                                        Entry::Vacant(entry) => {
                                            let PackageId::Cargo(package_id) = &package.id else {
                                                continue;
                                            };
                                            let Some(this_cargo_package) =
                                                self.cache.cargo_metadata.package(package_id)
                                            else {
                                                continue;
                                            };

                                            // TODO: handle error?
                                            match Package::new_dependency(
                                                this_cargo_package,
                                                package_name,
                                                None,
                                                &self.cache.cargo_metadata,
                                            ) {
                                                Ok(pkg) => {
                                                    entry.insert(pkg.clone());
                                                    pkg
                                                }
                                                Err(_) => continue,
                                            }
                                        }
                                    }
                                }
                            };
                            let path = &import_path.components;
                            self.resolve_in(package, false, path, name, results, condition);
                        }
                    }
                }
                syntax::ImportContent::Collection(imports) => {
                    for import in imports {
                        let import_path = import_path.clone().join(import.path.iter().cloned());
                        to_resolve.push((import_path, &import.content));
                    }
                }
            }
        }
    }
}

impl Resolver for CompilePackageResolver<'_> {
    fn resolve_item(
        &mut self,
        path_from: &[String],
        item_path: &syntax::ModulePath,
        item_name: &str,
    ) -> Vec<ResolvedItem> {
        let mut results = Vec::new();

        match &item_path.origin {
            syntax::PathOrigin::Absolute => {
                self.resolve_in(
                    self.package.clone(), // TODO: do not clone!
                    false,
                    &item_path.components,
                    item_name,
                    &mut results,
                    Conditional::True,
                );
            }
            syntax::PathOrigin::Relative(n) => {
                let include_imports = *n == 0 && item_path.components.is_empty();

                let to_keep = path_from.len().saturating_sub(*n);
                let path = path_from.iter().take(to_keep).cloned().collect::<Vec<_>>();

                self.resolve_in(
                    self.package.clone(), // TODO: do not clone!
                    include_imports,
                    &path,
                    item_name,
                    &mut results,
                    Conditional::True,
                );
            }
            syntax::PathOrigin::Package(pkg) => {
                // Resolve as pkg as suffix as one of the imports
                if let Some(syntax) = self.get_syntax(self.package.id.clone(), path_from) {
                    for import in &syntax.imports {
                        let item_path_and_name = [pkg.clone()]
                            .into_iter()
                            .chain(item_path.components.iter().cloned())
                            .chain([item_name.to_string()])
                            .collect::<Vec<_>>();
                        self.resolve_import(
                            &self.package.clone(), // TODO: do not clone!
                            import,
                            path_from,
                            &item_path_and_name,
                            &mut results,
                            Conditional::True,
                        );
                    }
                }

                // TODO: resolve as pkg as dependency
                // TODO: if one of the imports (or the "sum" of the imports) is "unconditional"
                //       this should not be considered, as this shadows any dependency usage?
            }
        }

        results
    }

    fn finish(self) -> ResolverResult {
        ResolverResult {
            version: self.package.version,
            dependencies: self
                .dependencies
                .into_iter()
                .map(|package| (package.package_name, package.version))
                .collect(),
        }
    }
}

// local_name -> package
enum Dependencies {
    Explicit {
        dependencies: HashMap<String, Package>,
    },
    Auto {
        dependencies: HashMap<String, Package>,
    },
}

impl Dependencies {
    fn into_iter(self) -> impl Iterator<Item = Package> {
        match self {
            Dependencies::Explicit { dependencies } => dependencies.into_values(),
            Dependencies::Auto { dependencies } => dependencies.into_values(),
        }
    }
}

fn compile_package(package: Package) -> Result<WeslModule> {
    Ok(WeslModule {
        name: package.package_name,
        code: None,
        submodules: compile_submodules(&package.root)?,
    })
}

fn compile_submodules(dir: &Path) -> Result<Vec<WeslModule>> {
    let mut submodules = HashMap::new();

    if !dir.is_dir() {
        return Ok(Vec::new());
    }

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
                    code: None,
                    submodules: Vec::new(),
                });

            let source = fs::read_to_string(&path)?;
            let syntax = self::preprocess::preprocess(wgsl_parse::parse_str(&source)?)?;

            sub.code = Some((syntax, source))
        } else if path.is_dir() {
            let sub = submodules
                .entry(name)
                .or_insert_with_key(|name| WeslModule {
                    name: name.clone(),
                    code: None,
                    submodules: Vec::new(),
                });
            sub.submodules = compile_submodules(&path)?;
        }
    }

    Ok(submodules
        .into_values()
        .filter(|module| module.code.is_some() || !module.submodules.is_empty())
        .collect())
}

fn decl_info(decl: &syntax::GlobalDeclaration) -> Option<(&syntax::Ident, ItemKind)> {
    match decl {
        syntax::GlobalDeclaration::Void => None,
        syntax::GlobalDeclaration::Declaration(declaration) => Some((
            &declaration.ident,
            match declaration.kind {
                syntax::DeclarationKind::Const => ItemKind::Constant,
                syntax::DeclarationKind::Override => ItemKind::Override,
                syntax::DeclarationKind::Let => return None,
                syntax::DeclarationKind::Var(_) => ItemKind::GlobalVariable,
            },
        )),
        syntax::GlobalDeclaration::TypeAlias(type_alias) => {
            Some((&type_alias.ident, ItemKind::TypeAlias))
        }
        syntax::GlobalDeclaration::Struct(s) => Some((&s.ident, ItemKind::Struct)),
        syntax::GlobalDeclaration::Function(function) => {
            Some((&function.ident, ItemKind::Function))
        }
        syntax::GlobalDeclaration::ConstAssert(_) => None,
        syntax::GlobalDeclaration::Compound(_) => None,
    }
}
