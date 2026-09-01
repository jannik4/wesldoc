use std::sync::LazyLock;
use syntect::{
    html::{ClassStyle, line_tokens_to_classed_spans},
    parsing::{
        ParseState, ScopeStack, SyntaxDefinition, SyntaxReference, SyntaxSet, SyntaxSetBuilder,
    },
};

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(|| {
    let mut builder = SyntaxSetBuilder::new();
    builder.add(
        SyntaxDefinition::load_from_str(include_str!("wgsl.sublime-syntax"), true, None).unwrap(),
    );
    builder.add(
        SyntaxDefinition::load_from_str(include_str!("wesl.sublime-syntax"), true, None).unwrap(),
    );
    builder.build()
});

static SYNTAX_WESL: LazyLock<&'static SyntaxReference> = LazyLock::new(|| {
    SYNTAX_SET
        .find_syntax_by_name("WESL")
        .expect("wesl syntax not found")
});

pub fn lines(source: &str) -> impl Iterator<Item = (usize, String)> + '_ {
    let mut parse_state = ParseState::new(&SYNTAX_WESL);
    let mut scope_stack = ScopeStack::new();
    let mut open_spans = 0;

    let mut lines = source.lines().enumerate().peekable();

    std::iter::from_fn(move || {
        let (line_idx, line) = lines.next()?;
        let is_last_line = lines.peek().is_none();

        let line = format!("{}\n", line);
        let mut line = match highlight_line(&line, &mut parse_state, &mut scope_stack) {
            Some((line, delta)) => {
                open_spans += delta;
                line
            }
            None => line,
        };

        if is_last_line {
            if line.ends_with('\n') {
                line.pop();
            }
            for _ in 0..open_spans {
                line.push_str("</span>");
            }
        }

        Some((line_idx + 1, line))
    })
}

fn highlight_line(
    line: &str,
    parse_state: &mut ParseState,
    scope_stack: &mut ScopeStack,
) -> Option<(String, isize)> {
    line_tokens_to_classed_spans(
        line,
        &parse_state.parse_line(line, &SYNTAX_SET).ok()?,
        ClassStyle::SpacedPrefixed { prefix: "syntax-" },
        scope_stack,
    )
    .ok()
}
