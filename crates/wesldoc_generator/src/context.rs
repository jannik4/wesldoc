use std::str::FromStr;

use wesldoc_ast::{
    Attribute, DefinitionPath, DocComment, Expression, Ident, IntraDocLink, ItemKind, Module, Span,
    TypeExpression, WeslDocs, md,
};

#[derive(Debug)]
pub struct Context<'a> {
    pub build_as_latest: bool,
    pub doc: &'a WeslDocs,
    pub module: &'a Module,
    module_path: ModulePath,
}

impl<'a> Context<'a> {
    pub fn new(build_as_latest: bool, doc: &'a WeslDocs) -> Self {
        Self {
            build_as_latest,
            doc,
            module: &doc.root,
            module_path: ModulePath {
                segments: vec![(
                    doc.root.name.clone(),
                    "index.html".to_string(),
                    ItemKind::Module,
                )],
                level: 0,
            },
        }
    }

    pub fn with_submodule(&self, module: &'a Module) -> Self {
        Self {
            build_as_latest: self.build_as_latest,
            doc: self.doc,
            module,
            module_path: self.module_path.extend(
                &module.name,
                "index.html",
                ItemKind::Module,
                true,
            ),
        }
    }

    pub fn with_item(&self, name: impl Into<String>, kind: ItemKind) -> Self {
        Self {
            build_as_latest: self.build_as_latest,
            doc: self.doc,
            module: self.module,
            module_path: self.module_path.extend(name, "#", kind, false),
        }
    }

    pub fn level(&self) -> usize {
        self.module_path.level
    }

    pub fn segments(&self) -> impl Iterator<Item = &(String, String, ItemKind)> {
        self.module_path.segments.iter()
    }

    pub fn root_url(&self) -> String {
        self.module_path.root_url()
    }

    pub fn source_url(&self, span: Option<Span>) -> String {
        self.module_path.source_url(span)
    }

    pub fn def_path_url(&self, name: &Ident, kind: &ItemKind, def_path: &DefinitionPath) -> String {
        self.module_path.def_path_url(name, kind, def_path)
    }

    pub fn render_attributes(
        &self,
        attributes: &[Attribute],
        new_line_indent: Option<usize>,
    ) -> String {
        if attributes.is_empty() {
            return "".to_string();
        }

        let sep = match new_line_indent {
            Some(indent) => {
                let mut sep = String::new();
                sep.push('\n');
                for _ in 0..indent {
                    sep.push(' ');
                }
                sep
            }
            None => " ".to_string(),
        };

        let mut result = String::new();
        for (idx, attr) in attributes.iter().enumerate() {
            if idx != 0 {
                if attributes.len() <= 3 {
                    result.push(' ');
                } else {
                    result.push_str(&sep);
                }
            }
            result.push_str(&crate::RenderAttributeTemplate { ctx: self, attr }.to_string());
        }

        result.push_str(&sep);

        result
    }

    pub fn render_expression(&self, expr: &Expression) -> String {
        crate::RenderExpressionTemplate { ctx: self, expr }.to_string()
    }

    pub fn render_type(&self, ty: &TypeExpression) -> String {
        crate::RenderTypeTemplate { ctx: self, ty }.to_string()
    }

    pub fn render_doc_comment(&self, comment: Option<&DocComment>) -> String {
        let mut output = String::new();
        if let Some(comment) = comment {
            output.push_str(r#"<div class="comment">"#);
            let md = {
                let mut md = String::new();
                md::html::push_html(
                    &mut md,
                    comment
                        .unsafe_full
                        .iter()
                        .cloned()
                        .map(|e| self.process_intra_doc_links(e)),
                );
                ammonia::clean(&md)
            };
            output.push_str(&md);
            output.push_str(r#"</div>"#);
        }
        output
    }

    pub fn render_doc_comment_short(&self, comment: Option<&DocComment>) -> String {
        let mut output = String::new();
        if let Some(comment) = comment {
            output.push_str(r#"<div class="comment-inline">"#);
            let md = {
                let mut md = String::new();
                md::html::push_html(
                    &mut md,
                    comment
                        .unsafe_short
                        .iter()
                        .cloned()
                        .map(|e| self.process_intra_doc_links(e)),
                );
                ammonia::clean(&md)
            };
            output.push_str(&md);
            output.push_str(r#"</div>"#);
        }
        output
    }

    pub fn render_doc_comment_short_no_links(comment: Option<&DocComment>) -> String {
        let mut output = String::new();
        if let Some(comment) = comment {
            output.push_str(r#"<div class="comment-inline">"#);
            let md = {
                let mut md = String::new();
                md::html::push_html(&mut md, comment.unsafe_short_no_links.iter().cloned());
                ammonia::clean(&md)
            };
            output.push_str(&md);
            output.push_str(r#"</div>"#);
        }
        output
    }

    fn process_intra_doc_links<'e>(&self, mut event: md::Event<'e>) -> md::Event<'e> {
        if let md::Event::Start(md::Tag::Link { dest_url, .. }) = &mut event {
            if let Ok(link) = IntraDocLink::from_str(dest_url) {
                *dest_url = self
                    .def_path_url(&link.name, &link.kind, &link.def_path)
                    .into();
            }
        }
        event
    }
}

#[derive(Debug)]
struct ModulePath {
    segments: Vec<(String, String, ItemKind)>,
    level: usize,
}

impl ModulePath {
    fn extend(
        &self,
        name: impl Into<String>,
        url: impl Into<String>,
        kind: ItemKind,
        is_child: bool,
    ) -> Self {
        Self {
            segments: self
                .segments
                .iter()
                .map(|(name, url, kind)| {
                    if is_child {
                        (name.clone(), format!("../{}", url), *kind)
                    } else {
                        (name.clone(), url.clone(), *kind)
                    }
                })
                .chain([(name.into(), url.into(), kind)])
                .collect(),
            level: if is_child { self.level + 1 } else { self.level },
        }
    }

    fn root_url(&self) -> String {
        (0..self.level + 3).map(|_| "../").collect::<String>()
    }

    fn source_url(&self, span: Option<Span>) -> String {
        let mut url = String::new();

        for _ in 0..self.level {
            url.push_str("../");
        }
        url.push_str("../src/");

        for (idx, (name, _, kind)) in self.segments.iter().enumerate() {
            let is_last = idx == self.segments.len() - 1;

            if is_last {
                if *kind == ItemKind::Module {
                    url.push_str(name);
                    url.push_str(".html");
                } else {
                    url.pop();
                    url.push_str(".html");
                }
            } else {
                url.push_str(name);
                url.push('/');
            }
        }

        if let Some(span) = span {
            url.push('#');
            url.push_str(&span.line_start.to_string());
            if span.line_start != span.line_end {
                url.push('-');
                url.push_str(&span.line_end.to_string());
            }
        }

        url
    }

    fn def_path_url(&self, name: &Ident, kind: &ItemKind, def_path: &DefinitionPath) -> String {
        let mut url = String::new();

        match def_path {
            DefinitionPath::Absolute(components) => {
                for _ in 0..self.level {
                    url.push_str("../");
                }
                for c in components {
                    url.push_str(c);
                    url.push('/');
                }
            }
            DefinitionPath::Package(dep, version, components) => {
                for _ in 0..self.level + 3 {
                    url.push_str("../");
                }
                url.push_str(dep);
                url.push('/');
                url.push_str(&version.to_string());
                url.push('/');
                url.push_str(dep);
                url.push('/');
                for c in components {
                    url.push_str(c);
                    url.push('/');
                }
            }
        }

        match *kind {
            ItemKind::Module => url.push_str("index.html"),
            ItemKind::Constant => url.push_str(&format!("const.{}.html", name.0)),
            ItemKind::Override => url.push_str(&format!("override.{}.html", name.0)),
            ItemKind::GlobalVariable => url.push_str(&format!("var.{}.html", name.0)),
            ItemKind::Struct => url.push_str(&format!("struct.{}.html", name.0)),
            ItemKind::Function => url.push_str(&format!("fn.{}.html", name.0)),
            ItemKind::TypeAlias => url.push_str(&format!("alias.{}.html", name.0)),
        }

        url
    }
}
