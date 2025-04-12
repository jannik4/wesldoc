use crate::Context;
use std::ops::Range;
use wesldoc_ast::*;

pub fn calculate_span(range: Range<usize>, ctx: &Context) -> Option<Span> {
    let source = ctx.default_source()?;

    let mut span_line_start = None;
    let mut span_line_end = None;
    let mut position = 0;
    for (idx, line) in source.split_inclusive('\n').enumerate() {
        let line_end = position + line.len();

        if span_line_start.is_none() && line_end > range.start {
            span_line_start = Some(idx + 1);
        }

        if line_end >= range.end {
            span_line_end = Some(idx + 1);
            break;
        }

        position += line.len();
    }

    Some(Span {
        line_start: span_line_start.unwrap_or(1),
        line_end: span_line_end.or(span_line_start).unwrap_or(1),
    })
}
