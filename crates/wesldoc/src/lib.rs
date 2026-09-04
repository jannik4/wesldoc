use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use std::path::PathBuf;
use wesldoc_compiler::MissingDocumentation;
use wesldoc_resolver::Resolver;

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
        // Resolve dependencies
        let resolver_backend = {
            let (backend_name, create_resolver_backend) =
                wesldoc_resolver_auto::resolver_backend(&self.package)
                    .with_context(|| "failed to determine package manager")?;

            wesldoc_report::spinner(
                &format!("Resolving {} dependencies", backend_name),
                create_resolver_backend,
            )?
        };
        let backend_packages = resolver_backend
            .iter_packages(self.max_dependency_depth())
            .collect::<Vec<_>>();
        wesldoc_report::info!(
            "Resolved" =>
            "{} {} package{}",
            backend_packages.len(),
            resolver_backend.name(),
            if backend_packages.len() == 1 { "" } else { "s" },
        );

        // Find wesl packages
        let mut packages = Vec::new();
        wesldoc_report::progress("Indexing", backend_packages.len() as u64, |pb| {
            for backend_package in backend_packages {
                // Package from backend package and check if it is a wesl package
                let package = backend_package.to_package()?;
                if package.is_wesl_package()? {
                    packages.push(package);
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
        let mut resolver = Resolver::new(&*resolver_backend);
        wesldoc_report::progress("Documenting", packages.len() as u64, |pb| {
            for package in packages {
                wesldoc_report::info!(
                    "Documenting" =>
                    "{} v{}",
                    package.package_name,
                    package.version
                );

                // Compile to docs
                let (docs, compile_stats) = wesldoc_compiler::compile(
                    &mut resolver,
                    &package.id,
                    package.version,
                    package.homepage,
                    package.repository,
                    package.license,
                    &wesldoc_compiler::CompileOptions {
                        missing_documentation: self.missing_docs.into(),
                    },
                )
                .with_context(|| format!("failed to compile package '{}'", package.package_name))?;
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
