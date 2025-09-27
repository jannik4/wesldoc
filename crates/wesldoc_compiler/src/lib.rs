mod build_attributes;
mod build_conditional;
mod build_doc_comment;
mod build_expression;
mod build_type;
mod calculate_span;
mod collect_features;
mod context;
mod extract_comments;
mod map;
mod post_process;

use self::{
    build_attributes::build_attributes,
    build_conditional::{ConditionalScope, build_conditional},
    build_doc_comment::{build_inner_doc_comment, build_outer_doc_comment},
    build_expression::build_expression,
    build_type::build_type,
    calculate_span::calculate_span,
    collect_features::collect_features,
    context::{Context, ResolveTarget},
    extract_comments::{extract_comments_inner, extract_comments_outer},
    map::map,
};
use std::collections::HashMap;
use wesl::{CompileResult, ModulePath, syntax};
use wesldoc_ast::*;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct WeslPackage {
    pub version: Version,
    pub dependencies: HashMap<String, (String, Version)>,
    pub root: WeslModule,
}

pub struct WeslModule {
    pub name: String,
    pub compiled: Option<(Vec<syntax::ImportStatement>, CompileResult)>,
    pub submodules: Vec<WeslModule>,
}

pub fn compile(package: &WeslPackage) -> Result<WeslDocs> {
    let mut docs = WeslDocs {
        version: package.version.clone(),
        root: compile_module(&package.root, &[], &package.dependencies)?,
    };

    post_process::post_process(&mut docs);

    Ok(docs)
}

fn compile_module(
    wesl_module: &WeslModule,
    path: &[String],
    dependencies: &HashMap<String, (String, Version)>,
) -> Result<Module> {
    let mut module = Module::empty(wesl_module.name.clone());
    module.modules = wesl_module
        .submodules
        .iter()
        .map(|m| {
            let mut path = path.to_vec();
            path.push(m.name.clone());
            compile_module(m, &path, dependencies)
        })
        .collect::<Result<Vec<_>>>()?;

    let Some((imports, compiled)) = &wesl_module.compiled else {
        return Ok(module);
    };
    let ctx = Context::init(
        imports,
        compiled,
        ModulePath {
            origin: syntax::PathOrigin::Absolute,
            components: path.to_vec(),
        },
        dependencies,
    );

    // Set source
    if let Some(source) = ctx.get_source() {
        module.source = Some(source.to_string());
    }

    // Set comment
    module.comment = module
        .source
        .as_ref()
        .and_then(|source| build_inner_doc_comment(&extract_comments_inner(source), &ctx));

    // Collect translate time features
    module.translate_time_features = collect_features(&ctx);

    // Compile locally defined global declarations and re-exports
    let mut conditional_scope = ConditionalScope::new();
    for decl in &compiled.syntax.global_declarations {
        let export_ctx;
        let mut export_conditional_scope;
        let (name, ctx, conditional_scope) = if let Some(name) = ctx.as_local(decl) {
            (name, &ctx, &mut conditional_scope)
        } else if let Some((module_path, name)) = ctx.as_export(decl) {
            // TODO: In the html output the source link is broken for re-exports.
            // It points to this module, instead of the module where the item is originally defined.

            // TODO: The empty conditional scope created below is not correct.
            // It should come from the module where the item is originally defined. For this we
            // need to compile/resolve that module too?

            export_ctx = ctx.at_path(module_path);
            export_conditional_scope = ConditionalScope::new();
            (name.clone(), &export_ctx, &mut export_conditional_scope)
        } else {
            continue;
        };

        let span = calculate_span(decl.span().range(), ctx);
        let comment = span
            .and_then(|span| Some((span, ctx.get_source()?)))
            .and_then(|(span, source)| {
                build_outer_doc_comment(&extract_comments_outer(span, source), ctx)
            });

        match decl.node() {
            syntax::GlobalDeclaration::Void => (),
            syntax::GlobalDeclaration::Declaration(declaration) => match declaration.kind {
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
                            conditional: build_conditional(
                                conditional_scope,
                                &declaration.attributes,
                            ),
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
                            conditional: build_conditional(
                                conditional_scope,
                                &declaration.attributes,
                            ),
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
                            conditional: build_conditional(
                                conditional_scope,
                                &declaration.attributes,
                            ),
                            comment,
                            span,
                        });
                }
            },
            syntax::GlobalDeclaration::TypeAlias(type_alias) => {
                module
                    .type_aliases
                    .entry(name.clone())
                    .or_default()
                    .instances
                    .push(TypeAlias {
                        name,
                        ty: build_type(&type_alias.ty, ctx),
                        attributes: build_attributes(&type_alias.attributes, ctx),
                        conditional: build_conditional(conditional_scope, &type_alias.attributes),
                        comment,
                        span,
                    });
            }
            syntax::GlobalDeclaration::Struct(struct_) => {
                module
                    .structs
                    .entry(name.clone())
                    .or_default()
                    .instances
                    .push(Struct {
                        name,
                        members: {
                            let mut conditional_scope = ConditionalScope::new();
                            struct_
                                .members
                                .iter()
                                .map(|member| StructMember {
                                    name: map(&member.ident),
                                    ty: build_type(&member.ty, ctx),
                                    attributes: build_attributes(&member.attributes, ctx),
                                    conditional: build_conditional(
                                        &mut conditional_scope,
                                        &member.attributes,
                                    ),
                                    comment: {
                                        calculate_span(member.span().range(), ctx)
                                            .and_then(|span| Some((span, ctx.get_source()?)))
                                            .and_then(|(span, source)| {
                                                build_outer_doc_comment(
                                                    &extract_comments_outer(span, source),
                                                    ctx,
                                                )
                                            })
                                    },
                                })
                                .collect()
                        },
                        attributes: build_attributes(&struct_.attributes, ctx),
                        conditional: build_conditional(conditional_scope, &struct_.attributes),
                        comment,
                        span,
                    });
            }
            syntax::GlobalDeclaration::Function(function) => {
                module
                    .functions
                    .entry(name.clone())
                    .or_default()
                    .instances
                    .push(Function {
                        name,
                        parameters: {
                            let mut conditional_scope = ConditionalScope::new();
                            function
                                .parameters
                                .iter()
                                .map(|param| FunctionParameter {
                                    name: map(&param.ident),
                                    ty: build_type(&param.ty, ctx),
                                    attributes: build_attributes(&param.attributes, ctx),
                                    conditional: build_conditional(
                                        &mut conditional_scope,
                                        &param.attributes,
                                    ),
                                })
                                .collect()
                        },
                        ret: function
                            .return_type
                            .as_ref()
                            .map(|ret| build_type(ret, ctx)),
                        attributes: build_attributes(&function.attributes, ctx),
                        return_attributes: build_attributes(&function.return_attributes, ctx),
                        conditional: build_conditional(conditional_scope, &function.attributes),
                        comment,
                        span,
                    });
            }
            syntax::GlobalDeclaration::ConstAssert(_const_assert) => (),
        }
    }

    Ok(module)
}
