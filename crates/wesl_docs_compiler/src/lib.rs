mod build_attributes;
mod build_conditional;
mod build_doc_comment;
mod build_expression;
mod build_type;
mod calculate_span;
mod collect_features;
mod extract_comments;
mod map;
mod post_process;
mod source_map;

use self::{
    build_attributes::build_attributes,
    build_conditional::{ConditionalScope, build_conditional},
    build_doc_comment::{build_inner_doc_comment, build_outer_doc_comment},
    build_expression::build_expression,
    build_type::build_type,
    calculate_span::calculate_span,
    collect_features::collect_features,
    extract_comments::{extract_comments_inner, extract_comments_outer},
    map::map,
    source_map::SourceMap,
};
use std::collections::HashMap;
use wesl::{CompileResult, syntax};
use wesl_docs::*;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct WeslPackage {
    pub version: Version,
    pub dependencies: HashMap<String, Version>,
    pub root: WeslModule,
}

pub struct WeslModule {
    pub name: String,
    pub compiled: Option<CompileResult>,
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
    dependencies: &HashMap<String, Version>,
) -> Result<Module> {
    let mut path = path.to_vec();
    path.push(wesl_module.name.clone());

    let mut module = Module::empty(wesl_module.name.clone());
    module.modules = wesl_module
        .submodules
        .iter()
        .map(|m| compile_module(m, &path, dependencies))
        .collect::<Result<Vec<_>>>()?;

    let Some(compiled) = &wesl_module.compiled else {
        return Ok(module);
    };
    if compiled.sourcemap.is_none() {
        println!(
            "Warning: No source map found for module {}",
            wesl_module.name
        );
    }
    let mut source_map = SourceMap::new(compiled);

    // Set source
    if let Some(source) = source_map.default_source() {
        module.source = Some(source.to_string());
    }

    // Set comment
    module.comment = module.source.as_ref().and_then(|source| {
        build_inner_doc_comment(
            &extract_comments_inner(source),
            &source_map,
            &path,
            dependencies,
        )
    });

    // Collect translate time features
    module.translate_time_features = collect_features(compiled);

    // Insert global declarations from this module as locals
    for decl in &compiled.syntax.global_declarations {
        match decl.node() {
            syntax::GlobalDeclaration::Void => (),
            syntax::GlobalDeclaration::Declaration(declaration) => match declaration.kind {
                syntax::DeclarationKind::Const => {
                    source_map.insert_local(&declaration.ident.name(), ItemKind::Constant);
                }
                syntax::DeclarationKind::Override => {
                    // TODO: ...
                }
                syntax::DeclarationKind::Let => (), // should be unreachable?
                syntax::DeclarationKind::Var(_) => {
                    source_map.insert_local(&declaration.ident.name(), ItemKind::GlobalVariable);
                }
            },
            syntax::GlobalDeclaration::TypeAlias(type_alias) => {
                source_map.insert_local(&type_alias.ident.name(), ItemKind::TypeAlias);
            }
            syntax::GlobalDeclaration::Struct(struct_) => {
                source_map.insert_local(&struct_.ident.name(), ItemKind::Struct);
            }
            syntax::GlobalDeclaration::Function(function) => {
                source_map.insert_local(&function.ident.name(), ItemKind::Function);
            }
            syntax::GlobalDeclaration::ConstAssert(_const_assert) => (),
        }
    }

    // Compile local global declarations
    let mut conditional_scope = ConditionalScope::new();
    for decl in &compiled.syntax.global_declarations {
        if !source_map.is_local(decl) {
            continue;
        }

        let span = calculate_span(decl.span().range(), &source_map);
        let comment = span
            .and_then(|span| Some((span, module.source.as_ref()?)))
            .and_then(|(span, source)| {
                build_outer_doc_comment(
                    &extract_comments_outer(span, source),
                    &source_map,
                    &path,
                    dependencies,
                )
            });

        match decl.node() {
            syntax::GlobalDeclaration::Void => (),
            syntax::GlobalDeclaration::Declaration(declaration) => match declaration.kind {
                syntax::DeclarationKind::Const => {
                    let name = map(&declaration.ident);
                    module
                        .constants
                        .entry(name.clone())
                        .or_default()
                        .instances
                        .push(Constant {
                            name,
                            ty: declaration
                                .ty
                                .as_ref()
                                .map(|ty| build_type(ty, &source_map, &path, dependencies)),
                            init: declaration
                                .initializer
                                .as_ref()
                                .map(|expr| {
                                    build_expression(expr, &source_map, &path, dependencies)
                                })
                                .unwrap_or(Expression::Unknown),
                            attributes: build_attributes(
                                &declaration.attributes,
                                &source_map,
                                &path,
                                dependencies,
                            ),
                            conditional: build_conditional(
                                &mut conditional_scope,
                                &declaration.attributes,
                            ),
                            comment,
                            span,
                        });
                }
                syntax::DeclarationKind::Override => {
                    // TODO: ...
                }
                syntax::DeclarationKind::Let => (), // should be unreachable?
                syntax::DeclarationKind::Var(address_space) => {
                    let address_space = address_space.unwrap_or(syntax::AddressSpace::Handle);
                    let name = map(&declaration.ident);
                    module
                        .global_variables
                        .entry(name.clone())
                        .or_default()
                        .instances
                        .push(GlobalVariable {
                            name,
                            space: map(&address_space),
                            ty: declaration
                                .ty
                                .as_ref()
                                .map(|ty| build_type(ty, &source_map, &path, dependencies)),
                            init: declaration.initializer.as_ref().map(|expr| {
                                build_expression(expr, &source_map, &path, dependencies)
                            }),
                            attributes: build_attributes(
                                &declaration.attributes,
                                &source_map,
                                &path,
                                dependencies,
                            ),
                            conditional: build_conditional(
                                &mut conditional_scope,
                                &declaration.attributes,
                            ),
                            comment,
                            span,
                        });
                }
            },
            syntax::GlobalDeclaration::TypeAlias(type_alias) => {
                let name = map(&type_alias.ident);
                module
                    .type_aliases
                    .entry(name.clone())
                    .or_default()
                    .instances
                    .push(TypeAlias {
                        name,
                        ty: build_type(&type_alias.ty, &source_map, &path, dependencies),
                        attributes: build_attributes(
                            &type_alias.attributes,
                            &source_map,
                            &path,
                            dependencies,
                        ),
                        conditional: build_conditional(
                            &mut conditional_scope,
                            &type_alias.attributes,
                        ),
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
                            let mut conditional_scope = ConditionalScope::new();
                            struct_
                                .members
                                .iter()
                                .map(|member| StructMember {
                                    name: map(&member.ident),
                                    ty: build_type(&member.ty, &source_map, &path, dependencies),
                                    attributes: build_attributes(
                                        &member.attributes,
                                        &source_map,
                                        &path,
                                        dependencies,
                                    ),
                                    conditional: build_conditional(
                                        &mut conditional_scope,
                                        &member.attributes,
                                    ),
                                    comment: None, // TODO: Needs struct member span or comments from wesl-rs
                                })
                                .collect()
                        },
                        attributes: build_attributes(
                            &struct_.attributes,
                            &source_map,
                            &path,
                            dependencies,
                        ),
                        conditional: build_conditional(&mut conditional_scope, &struct_.attributes),
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
                            let mut conditional_scope = ConditionalScope::new();
                            function
                                .parameters
                                .iter()
                                .map(|param| FunctionParameter {
                                    name: map(&param.ident),
                                    ty: build_type(&param.ty, &source_map, &path, dependencies),
                                    attributes: build_attributes(
                                        &param.attributes,
                                        &source_map,
                                        &path,
                                        dependencies,
                                    ),
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
                            .map(|ret| build_type(ret, &source_map, &path, dependencies)),
                        attributes: build_attributes(
                            &function.attributes,
                            &source_map,
                            &path,
                            dependencies,
                        ),
                        return_attributes: build_attributes(
                            &function.return_attributes,
                            &source_map,
                            &path,
                            dependencies,
                        ),
                        conditional: build_conditional(
                            &mut conditional_scope,
                            &function.attributes,
                        ),
                        comment,
                        span,
                    });
            }
            syntax::GlobalDeclaration::ConstAssert(_const_assert) => (),
        }
    }

    Ok(module)
}
