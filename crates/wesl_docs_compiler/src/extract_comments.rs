use wesl_docs::*;

pub fn extract_comments_inner(source: &str) -> String {
    let mut comments = String::new();

    for line in source.lines() {
        let line = line.trim_start();
        if line.starts_with("//!") {
            comments.push_str(line);
            comments.push('\n');
        } else if !line.is_empty() {
            break;
        }
    }

    comments
}

pub fn extract_comments_outer(item_span: Span, source: &str) -> String {
    let mut comments = String::new();

    let line_count = source.lines().count();

    for (idx, line) in source.lines().rev().enumerate() {
        let line_nr = line_count - idx;
        if line_nr >= item_span.line_start {
            continue;
        }

        let line = line.trim_start();
        if line.starts_with("///") {
            comments = format!("{}\n{}", line, comments);
        } else if !line.is_empty() {
            break;
        }
    }

    comments
}
