use std::sync::LazyLock;
use tree_sitter_highlight::{Error, HighlightConfiguration, Highlighter, HtmlRenderer};

const HIGHLIGHT_NAMES: &[&str] = &[
    "comment.line",
    "comment.block",
    "variable",
    "variable.parameter",
    "type",
    "variable.other.member",
    "constant",
    "namespace",
    "function",
    "function.call",
    "punctuation",
    "keyword.storage.modifier",
    "attribute",
    "variable.builtin",
    "constant.builtin.boolean",
    "constant.numeric.integer",
    "constant.numeric.float",
    "keyword.control",
    "keyword.control.function",
    "keyword.control.conditional",
    "keyword.control.repeat",
    "keyword.control.return",
    "keyword.storage.type",
    "keyword.directive",
    "keyword.control.import",
    "operator",
    "punctuation.bracket",
    "punctuation.delimiter",
];

static HIGHLIGHT_CLASSES: LazyLock<Vec<Vec<String>>> = LazyLock::new(|| {
    HIGHLIGHT_NAMES
        .iter()
        .map(|name| {
            let mut classes = Vec::new();
            let mut prefix = String::new();
            for s in name.split('.') {
                prefix.push_str(if prefix.is_empty() { "" } else { "-" });
                prefix.push_str(s);
                classes.push(format!("syntax-{}", prefix));
            }
            classes
        })
        .collect()
});

static WESL_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    let mut wesl_config = HighlightConfiguration::new(
        tree_sitter_wesl::LANGUAGE.into(),
        "WESL",
        tree_sitter_wesl::HIGHLIGHTS_QUERY,
        tree_sitter_wesl::INJECTIONS_QUERY,
        tree_sitter_wesl::LOCALS_QUERY,
    )
    .unwrap();
    wesl_config.configure(HIGHLIGHT_NAMES);
    wesl_config
});

pub fn lines(source: &str) -> Vec<(usize, String)> {
    let mut lines = match try_highlight(source) {
        Ok(lines) => lines,
        Err(err) => {
            wesldoc_report::warn!("Failed to highlight source code: {:?}", err);

            source
                .lines()
                .enumerate()
                .map(|(i, line)| (i + 1, line.to_string()))
                .collect()
        }
    };

    if let Some((_, last_line)) = lines.last_mut() {
        *last_line = last_line.trim_end_matches(['\r', '\n']).to_string();
    }

    lines
}

fn try_highlight(source: &str) -> Result<Vec<(usize, String)>, Error> {
    let mut highlighter = Highlighter::new();
    let highlights =
        highlighter.highlight(&WESL_CONFIG, source.as_bytes(), None, None, |_| None)?;

    let mut html_renderer = HtmlRenderer::new();
    html_renderer.render(highlights, source.as_bytes(), &|highlight, bytes| {
        bytes.extend_from_slice(b"class=\"");
        for (idx, class) in HIGHLIGHT_CLASSES[highlight.0].iter().enumerate() {
            if idx != 0 {
                bytes.extend_from_slice(b" ");
            }
            bytes.extend_from_slice(class.as_bytes());
        }
        bytes.extend_from_slice(b"\"");
    })?;

    Ok(html_renderer
        .lines()
        .enumerate()
        .map(|(i, line)| (i + 1, line.to_string()))
        .collect())
}
