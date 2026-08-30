mod build_attributes;
mod build_doc_comment;
mod build_expression;
mod build_type;
mod calculate_span;
mod collect_features;
mod compile_state;
mod context;
mod extract_comments;
mod map;
mod post_process;

pub mod build_conditional;

use self::{
    build_attributes::build_attributes,
    build_conditional::conditional_from_attributes,
    build_doc_comment::{build_inner_doc_comment, build_outer_doc_comment},
    build_expression::build_expression,
    build_type::build_type,
    calculate_span::calculate_span,
    collect_features::collect_features,
    compile_state::{CompileState, CompileStats},
    context::Context,
    extract_comments::{extract_comments_inner, extract_comments_outer},
    map::map,
};
use std::sync::Arc;
use thiserror::Error;
use wesldoc_ast::*;
use wgsl_parse::syntax::{self, ModulePath, TranslationUnit};

pub const ATTRIBUTE_CONDITIONAL: &str = "computed_cfg";

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolveItemKind {
    Declaration,
    DeclarationOrModule,
}

#[derive(Debug, Clone)]
pub struct ResolvedItem {
    pub kind: ItemKind,
    pub def_path: DefinitionPath,
    pub name: Ident,
    pub conditional: Option<Conditional>,
}

pub trait Resolver {
    type PackageId;

    fn resolve_item(
        &mut self,
        package_from: &Self::PackageId,
        path_from: &[String],
        item_path: &ModulePath,
        item_kind: ResolveItemKind,
    ) -> Vec<ResolvedItem>;

    fn resolve_dependency(
        &mut self,
        package_from: &Self::PackageId,
        dependency_name: &str,
    ) -> Option<Self::PackageId>;

    fn resolved_dependencies(&self, package_id: &Self::PackageId) -> Vec<(String, Version)>;
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("package has missing documentation")]
    MissingDocumentation,
}

impl From<FatalError> for Error {
    fn from(e: FatalError) -> Self {
        match e {}
    }
}

#[derive(Debug, Error)]
enum FatalError {}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissingDocumentation {
    #[default]
    Allow,
    Warn,
    Deny,
}

#[derive(Debug, Default, Clone)]
pub struct CompileOptions {
    pub missing_documentation: MissingDocumentation,
}

pub struct WeslModule {
    pub name: String,
    pub code: Option<(Arc<TranslationUnit>, String)>,
    pub submodules: Vec<WeslModule>,
}

pub fn compile<T>(
    mut resolver: impl Resolver<PackageId = T>,
    package_id: &T,
    package_version: Version,
    root: &WeslModule,
    options: &CompileOptions,
) -> Result<(WeslDocs, CompileStats)> {
    let compile_state = CompileState::default();
    let root = compile_module(
        &mut resolver,
        package_id,
        root,
        &[],
        options,
        &compile_state,
    )?;
    let mut docs = WeslDocs {
        version: package_version,
        dependencies: resolver.resolved_dependencies(package_id),
        root,
    };
    let compile_stats = compile_state.into_result()?;

    post_process::post_process(&mut docs);

    Ok((docs, compile_stats))
}

fn compile_module<T>(
    resolver: &mut dyn Resolver<PackageId = T>,
    package_id: &T,
    wesl_module: &WeslModule,
    path: &[String],
    compile_options: &CompileOptions,
    compile_state: &CompileState,
) -> Result<Module, FatalError> {
    let mut module = Module::empty(wesl_module.name.clone());
    module.modules = wesl_module
        .submodules
        .iter()
        .map(|m| {
            let mut path = path.to_vec();
            path.push(m.name.clone());
            compile_module(
                resolver,
                package_id,
                m,
                &path,
                compile_options,
                compile_state,
            )
        })
        .collect::<Result<Vec<_>, FatalError>>()?;

    let Some((syntax, source)) = &wesl_module.code else {
        return Ok(module);
    };
    let ctx = &Context::init(
        resolver,
        package_id,
        path,
        syntax,
        source,
        compile_options,
        compile_state,
    );

    // Set source
    module.source = Some(source.clone());

    // Set comment
    module.comment = module
        .source
        .as_ref()
        .and_then(|source| build_inner_doc_comment(&extract_comments_inner(source), ctx));
    validate_module_doc_comment(&module, ctx);

    // Collect translate time features
    module.translate_time_features = collect_features(ctx);

    // TODO(no-comp): Compile re-exports (collect from imports)
    // --> create a new context then

    // Compile locally defined global declarations
    compile_decls(&syntax.global_declarations, ctx, &mut module)?;

    Ok(module)
}

fn compile_decls<T>(
    decls: &[syntax::GlobalDeclarationNode],
    ctx: &Context<T>,
    module: &mut Module,
) -> Result<(), FatalError> {
    for decl in decls {
        let span = calculate_span(decl.span().range(), ctx);
        let comment = span.and_then(|span| {
            build_outer_doc_comment(&extract_comments_outer(span, ctx.source()), ctx)
        });
        validate_item_doc_comment(&comment, decl.span(), ctx);

        match decl.node() {
            syntax::GlobalDeclaration::Void => (),
            syntax::GlobalDeclaration::Compound(compound) => {
                compile_decls(&compound.body, ctx, module)?;
            }
            syntax::GlobalDeclaration::Declaration(declaration) => {
                let name = map(&declaration.ident);
                match declaration.kind {
                    syntax::DeclarationKind::Const => {
                        module
                            .constants
                            .entry(name.clone())
                            .or_default()
                            .instances
                            .push(Constant {
                                name,
                                ty: declaration.ty.as_ref().map(|ty| build_type(ty, ctx)),
                                init: declaration
                                    .initializer
                                    .as_ref()
                                    .map(|expr| build_expression(expr, ctx))
                                    .unwrap_or(Expression::NotExpanded(None)),
                                attributes: build_attributes(&declaration.attributes, ctx),
                                conditional: conditional_from_attributes(&declaration.attributes),
                                comment,
                                span,
                            });
                    }
                    syntax::DeclarationKind::Override => {
                        module
                            .overrides
                            .entry(name.clone())
                            .or_default()
                            .instances
                            .push(Override {
                                name,
                                ty: declaration.ty.as_ref().map(|ty| build_type(ty, ctx)),
                                init: declaration
                                    .initializer
                                    .as_ref()
                                    .map(|expr| build_expression(expr, ctx)),
                                attributes: build_attributes(&declaration.attributes, ctx),
                                conditional: conditional_from_attributes(&declaration.attributes),
                                comment,
                                span,
                            });
                    }
                    syntax::DeclarationKind::Let => (), // should be unreachable?
                    syntax::DeclarationKind::Var(address_space) => {
                        let address_space =
                            address_space.unwrap_or((syntax::AddressSpace::Handle, None));
                        module
                            .global_variables
                            .entry(name.clone())
                            .or_default()
                            .instances
                            .push(GlobalVariable {
                                name,
                                space: map(&address_space),
                                ty: declaration.ty.as_ref().map(|ty| build_type(ty, ctx)),
                                init: declaration
                                    .initializer
                                    .as_ref()
                                    .map(|expr| build_expression(expr, ctx)),
                                attributes: build_attributes(&declaration.attributes, ctx),
                                conditional: conditional_from_attributes(&declaration.attributes),
                                comment,
                                span,
                            });
                    }
                }
            }
            syntax::GlobalDeclaration::TypeAlias(type_alias) => {
                let name = map(&type_alias.ident);
                module
                    .type_aliases
                    .entry(name.clone())
                    .or_default()
                    .instances
                    .push(TypeAlias {
                        name,
                        ty: build_type(&type_alias.ty, ctx),
                        attributes: build_attributes(&type_alias.attributes, ctx),
                        conditional: conditional_from_attributes(&type_alias.attributes),
                        comment,
                        span,
                    });
            }
            syntax::GlobalDeclaration::Struct(struct_) => {
                let name = map(&struct_.ident);
                module
                    .structs
                    .entry(name.clone())
                    .or_default()
                    .instances
                    .push(Struct {
                        name,
                        members: {
                            struct_
                                .members
                                .iter()
                                .map(|member| StructMember {
                                    name: map(&member.ident),
                                    ty: build_type(&member.ty, ctx),
                                    attributes: build_attributes(&member.attributes, ctx),
                                    conditional: conditional_from_attributes(&member.attributes),
                                    comment: {
                                        let comment = calculate_span(member.span().range(), ctx)
                                            .and_then(|span| {
                                                build_outer_doc_comment(
                                                    &extract_comments_outer(span, ctx.source()),
                                                    ctx,
                                                )
                                            });
                                        validate_item_doc_comment(&comment, member.span(), ctx);
                                        comment
                                    },
                                })
                                .collect()
                        },
                        attributes: build_attributes(&struct_.attributes, ctx),
                        conditional: conditional_from_attributes(&struct_.attributes),
                        comment,
                        span,
                    });
            }
            syntax::GlobalDeclaration::Function(function) => {
                let name = map(&function.ident);
                module
                    .functions
                    .entry(name.clone())
                    .or_default()
                    .instances
                    .push(Function {
                        name,
                        parameters: {
                            function
                                .parameters
                                .iter()
                                .map(|param| FunctionParameter {
                                    name: map(&param.ident),
                                    ty: build_type(&param.ty, ctx),
                                    attributes: build_attributes(&param.attributes, ctx),
                                    conditional: conditional_from_attributes(&param.attributes),
                                })
                                .collect()
                        },
                        ret: function
                            .return_type
                            .as_ref()
                            .map(|ret| build_type(ret, ctx)),
                        attributes: build_attributes(&function.attributes, ctx),
                        return_attributes: build_attributes(&function.return_attributes, ctx),
                        conditional: conditional_from_attributes(&function.attributes),
                        comment,
                        span,
                    });
            }
            syntax::GlobalDeclaration::ConstAssert(_const_assert) => (),
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Severity {
    Warn,
    Error,
}

impl Severity {
    fn to_miette_severity(self) -> miette::Severity {
        match self {
            Severity::Warn => miette::Severity::Warning,
            Severity::Error => miette::Severity::Error,
        }
    }
}

fn validate_module_doc_comment<T>(module: &Module, ctx: &Context<T>) {
    let is_documented = module.comment.is_some();
    ctx.compile_state().track_documented(is_documented);
    if is_documented {
        return;
    }
    let severity = match ctx.compile_options().missing_documentation {
        MissingDocumentation::Allow => return,
        MissingDocumentation::Warn => Severity::Warn,
        MissingDocumentation::Deny => Severity::Error,
    };
    let mut report = miette::miette!(
        severity = severity.to_miette_severity(),
        "missing module documentation for module `{}`",
        module.name
    );
    report = report.with_source_code(ctx.source().to_string());
    match severity {
        Severity::Warn => {
            log::warn!("{report:?}");
        }
        Severity::Error => {
            log::error!("{report:?}");
            ctx.compile_state()
                .report_error(Error::MissingDocumentation);
        }
    }
}

fn validate_item_doc_comment<T>(
    comment: &Option<DocComment>,
    span: wgsl_parse::syntax::Span,
    ctx: &Context<T>,
) {
    let is_documented = comment.is_some();
    ctx.compile_state().track_documented(is_documented);
    if is_documented {
        return;
    }
    let severity = match ctx.compile_options().missing_documentation {
        MissingDocumentation::Allow => return,
        MissingDocumentation::Warn => Severity::Warn,
        MissingDocumentation::Deny => Severity::Error,
    };
    let mut report = miette::miette!(
        labels = vec![miette::LabeledSpan::at(
            span.range(),
            "missing documentation"
        )],
        severity = severity.to_miette_severity(),
        "missing item documentation"
    );
    report = report.with_source_code(ctx.source().to_string());
    match severity {
        Severity::Warn => {
            log::warn!("{report:?}");
        }
        Severity::Error => {
            log::error!("{report:?}");
            ctx.compile_state()
                .report_error(Error::MissingDocumentation);
        }
    }
}
