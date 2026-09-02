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
    pub fn run(self) {
        if let Err(e) = self.try_run() {
            wesldoc_report::error!("{:?}", e);
            std::process::exit(1);
        }
    }

    fn try_run(self) -> Result<()> {
        // Check Cargo.toml exist
        if !self.package.join("Cargo.toml").is_file() {
            bail!("Cargo.toml not found");
        }

        // Resolve cargo dependencies
        let cargo_metadata = Arc::new(wesldoc_report::spinner(
            "Resolving cargo dependencies",
            || CargoMetadata::resolve(&self.package),
        )?);
        let cargo_packages = cargo_metadata
            .iter_packages(self.max_dependency_depth())
            .collect::<Vec<_>>();
        wesldoc_report::info!(
            "Resolved" =>
            "{} cargo package{}",
            cargo_packages.len(),
            if cargo_packages.len() == 1 { "" } else { "s" },
        );

        // Find wesl packages
        let mut packages = Vec::new();
        wesldoc_report::progress("Indexing", cargo_packages.len() as u64, |pb| {
            for cargo_package in cargo_packages {
                // Package from cargo package and check if it is a wesl package
                let package = Package::from_cargo_package(cargo_package)?;
                if package.is_wesl_package()? {
                    packages.push((package, cargo_package));
                }

                pb.inc(1);
            }

            Ok::<_, anyhow::Error>(())
        })?;
        if packages.is_empty() {
            bail!("No wesl packages found in the specified path");
        }
        wesldoc_report::info!(
            "Indexed" =>
            "{} wesl package{}",
            packages.len(),
            if packages.len() == 1 { "" } else { "s" },
        );

        // Document packages
        let mut cache = BuildCache::new(Arc::clone(&cargo_metadata));
        wesldoc_report::progress("Documenting", packages.len() as u64, |pb| {
            for (package, cargo_package) in packages {
                wesldoc_report::info!(
                    "Documenting" =>
                    "{} v{}",
                    package.package_name,
                    package.version
                );

                // Build wesl module
                let wesl_module = Arc::clone(
                    &cache
                        .get_or_build(package.id.clone())?
                        .context("expected wesl package")?
                        .build,
                );

                // Compile to docs
                let resolver =
                    CompilePackageResolver::new(&mut cache, Arc::clone(&cargo_metadata))?;
                let (docs, compile_stats) = wesldoc_compiler::compile(
                    resolver,
                    &package.id,
                    package.version,
                    &wesl_module,
                    cargo_package.homepage(),
                    cargo_package.repository(),
                    cargo_package.license(),
                    &wesldoc_compiler::CompileOptions {
                        missing_documentation: self.missing_docs.into(),
                    },
                )
                .with_context(|| format!("failed to compile package '{}'", wesl_module.name))?;
                if self.statistics {
                    wesldoc_report::metric!(
                        "Coverage" =>
                        "{:.2}%",
                        compile_stats.documented_percentage()
                    );
                }

                // Generate docs
                wesldoc_generator::generate(&docs, &self.output)?;

                pb.inc(1);
            }

            Ok::<(), anyhow::Error>(())
        })?;

        wesldoc_report::info!(
            "Success" =>
            "Wrote documentation to {}",
            self.output.display()
        );

        Ok(())
    }

    fn max_dependency_depth(&self) -> usize {
        match self.no_deps {
            true => 0,
            false => self.max_dependency_depth.unwrap_or(usize::MAX),
        }
    }
}
