mod build;
mod cargo;
mod package;
mod resolver;
mod wesl_toml;

use self::{
    build::BuildCache, cargo::CargoMetadata, package::Package, resolver::CompilePackageResolver,
};
use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use std::{path::PathBuf, sync::Arc};
use wesldoc_compiler::MissingDocumentation;

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

impl Args {
    pub fn run(self) -> Result<()> {
        // Check Cargo.toml exist
        if !self.package.join("Cargo.toml").is_file() {
            bail!("Cargo.toml not found");
        }

        // Resolve cargo dependencies
        let cargo_metadata = Arc::new(CargoMetadata::resolve(&self.package)?);

        // Cache
        let mut cache = BuildCache::new(Arc::clone(&cargo_metadata));

        // Doc packages
        let mut wesl_package_found = false;
        for cargo_package in cargo_metadata.iter_packages(self.max_dependency_depth()) {
            // Package from cargo package and check if it is a wesl package
            let package = Package::from_cargo_package(cargo_package)?;
            if !package.is_wesl_package()? {
                continue;
            }
            wesl_package_found = true;

            println!(
                "Documenting package: {} v{}",
                package.package_name, package.version
            );

            // Build wesl module
            let wesl_module = Arc::clone(
                &cache
                    .get_or_build(package.id.clone())?
                    .context("expected wesl package")?
                    .build,
            );

            // Compile to docs
            let resolver = CompilePackageResolver::new(&mut cache, Arc::clone(&cargo_metadata))?;
            let (docs, compile_stats) = wesldoc_compiler::compile(
                resolver,
                &package.id,
                package.version,
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

    fn max_dependency_depth(&self) -> usize {
        match self.no_deps {
            true => 0,
            false => self.max_dependency_depth.unwrap_or(usize::MAX),
        }
    }
}
