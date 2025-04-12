mod resolver;

use self::resolver::DocsResolver;
use clap::Parser;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use wesl::{CompileOptions, Feature, Features, ManglerKind, Wesl};
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

    /// The path to a dependency. This can be used multiple times.
    #[arg(short, long)]
    dependency: Vec<PathBuf>,
}

impl Args {
    pub fn run(self) -> Result<()> {
        // Read package
        let package = Package::from_dir(&self.package)?;

        // Read dependencies
        let dependencies = self
            .dependency
            .iter()
            .map(|dep| {
                let package = Package::from_dir(dep)?;
                Ok(package)
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
                keep: None,
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
            .map(|dep| (dep.name.clone(), dep.version.clone()))
            .collect(),
        root: WeslModule {
            name: package.name,
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

        let name = name_from_path(&path);

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
            sub.compiled = Some(wesl.compile(path.strip_prefix(root)?)?)
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
    name: String,
    root: PathBuf,
    version: Version,
}

impl Package {
    fn from_dir(dir: &Path) -> Result<Self> {
        let dir = dir.canonicalize()?;

        Ok(Self {
            name: name_from_path(&dir),
            root: dir,
            version: Version::new(0, 0, 0),
        })
    }
}

fn name_from_path(path: &Path) -> String {
    path.file_stem()
        .unwrap()
        .to_string_lossy()
        .replace('-', "_")
}
