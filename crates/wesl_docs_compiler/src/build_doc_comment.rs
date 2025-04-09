use crate::source_map::SourceMap;
use std::collections::HashMap;
use wesl_docs::*;

pub fn build_inner_doc_comment(
    raw_comment: &str,
    source_map: &SourceMap,
    module_path: &[String],
    dependencies: &HashMap<String, Version>,
) -> Option<DocComment> {
    build_doc_comment(raw_comment, "//!", source_map, module_path, dependencies)
}

pub fn build_outer_doc_comment(
    raw_comment: &str,
    source_map: &SourceMap,
    module_path: &[String],
    dependencies: &HashMap<String, Version>,
) -> Option<DocComment> {
    build_doc_comment(raw_comment, "///", source_map, module_path, dependencies)
}

fn build_doc_comment(
    raw_comment: &str,
    comment_prefix: &str,
    _source_map: &SourceMap,
    _module_path: &[String],
    _dependencies: &HashMap<String, Version>,
) -> Option<DocComment> {
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
    let mut full = md::Parser::new(&comment)
        .map(|event| event.into_static())
        .collect::<Vec<_>>();
    if full.is_empty() {
        return None;
    }

    // Raise heading levels
    raise_heading_levels(&mut full);

    // TODO: Support intra-doc links

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

    Some(DocComment {
        unsafe_short: short,
        unsafe_full: full,
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
