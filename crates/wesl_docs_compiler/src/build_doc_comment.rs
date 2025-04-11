use crate::{Context, ResolveTarget};
use wesl_docs::*;

pub fn build_inner_doc_comment(raw_comment: &str, ctx: &Context) -> Option<DocComment> {
    build_doc_comment(raw_comment, "//!", ctx)
}

pub fn build_outer_doc_comment(raw_comment: &str, ctx: &Context) -> Option<DocComment> {
    build_doc_comment(raw_comment, "///", ctx)
}

fn build_doc_comment(raw_comment: &str, comment_prefix: &str, ctx: &Context) -> Option<DocComment> {
    // Strip the comment prefix
    let comment = raw_comment
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            line.starts_with(comment_prefix).then(|| &line[3..])
        })
        .enumerate()
        .fold(String::new(), |mut acc, (idx, line)| {
            if idx != 0 {
                acc.push('\n');
            }
            acc.push_str(line);
            acc
        });

    // Parse
    let mut full = md::Parser::new_with_broken_link_callback(
        &comment,
        md::Options::empty(),
        Some(|link: md::BrokenLink<'_>| {
            let trimmed = link.reference.trim_matches('`').to_string();
            Some((md::CowStr::from(trimmed.clone()), md::CowStr::from(trimmed)))
        }),
    )
    .map(|event| event.into_static())
    .collect::<Vec<_>>();
    if full.is_empty() {
        return None;
    }

    // Raise heading levels
    raise_heading_levels(&mut full);

    // Intra-doc links
    resolve_intra_doc_links(&mut full, ctx);

    // Create the short variant
    let mut complete = false;
    let mut balance = 0;
    let short = full
        .iter()
        .filter_map(|event| match event {
            md::Event::Start(tag) => {
                if complete {
                    return None;
                }

                balance += 1;
                Some(md::Event::Start(match tag {
                    md::Tag::Heading { .. } => md::Tag::Paragraph,
                    _ => tag.clone(),
                }))
            }
            md::Event::End(tag_end) => {
                if complete {
                    return None;
                }

                balance -= 1;
                complete |= balance == 0;
                Some(md::Event::End(match tag_end {
                    md::TagEnd::Heading(_) => md::TagEnd::Paragraph,
                    _ => *tag_end,
                }))
            }
            e @ md::Event::Text(_) => (!complete).then(|| e.clone()),
            e @ md::Event::Code(_) => (!complete).then(|| e.clone()),
            e @ md::Event::InlineMath(_) => (!complete).then(|| e.clone()),
            e @ md::Event::DisplayMath(_) => (!complete).then(|| e.clone()),
            e @ md::Event::Html(_) => (!complete).then(|| e.clone()),
            e @ md::Event::InlineHtml(_) => (!complete).then(|| e.clone()),
            e @ md::Event::FootnoteReference(_) => Some(e.clone()),
            e @ md::Event::SoftBreak => (!complete).then(|| e.clone()),
            md::Event::HardBreak | md::Event::Rule | md::Event::TaskListMarker(_) => {
                complete |= balance == 0;
                None
            }
        })
        .collect::<Vec<_>>();

    // Create the short no links variant
    let short_no_links = short
        .iter()
        .filter_map(|event| match event {
            md::Event::Start(md::Tag::Link { .. }) => None,
            md::Event::End(md::TagEnd::Link) => None,
            _ => Some(event.clone()),
        })
        .collect::<Vec<_>>();

    Some(DocComment {
        unsafe_full: full,
        unsafe_short: short,
        unsafe_short_no_links: short_no_links,
    })
}

fn raise_heading_levels(events: &mut [md::Event]) {
    for event in events {
        match event {
            md::Event::Start(md::Tag::Heading { level, .. }) => {
                *level = raise_heading_level(*level);
            }
            md::Event::End(md::TagEnd::Heading(level)) => {
                *level = raise_heading_level(*level);
            }
            _ => {}
        }
    }
}

fn raise_heading_level(level: md::HeadingLevel) -> md::HeadingLevel {
    match level {
        md::HeadingLevel::H1 => md::HeadingLevel::H2,
        md::HeadingLevel::H2 => md::HeadingLevel::H3,
        md::HeadingLevel::H3 => md::HeadingLevel::H4,
        md::HeadingLevel::H4 => md::HeadingLevel::H5,
        md::HeadingLevel::H5 => md::HeadingLevel::H6,
        md::HeadingLevel::H6 => md::HeadingLevel::H6,
    }
}

// TODO: Only works for items in scope as this just looks up the name in the source map
fn resolve_intra_doc_links(events: &mut [md::Event], ctx: &Context) {
    for event in events {
        if let md::Event::Start(md::Tag::Link { dest_url, .. }) = event {
            if let Some((name, kind, def_path)) =
                ctx.resolve_reference(ResolveTarget::Name(dest_url.as_ref()))
            {
                *dest_url = IntraDocLink {
                    def_path,
                    kind,
                    name,
                }
                .to_string()
                .into();
            } else {
                dbg!("Failed to resolve intra-doc link: {}", dest_url);
            }
        }
    }
}
