#[derive(Clone, Copy, Debug)]
pub enum DiagnosticKind {
    Lex,
    Parse,
    Semantic,
}

impl DiagnosticKind {
    fn as_str(self) -> &'static str {
        match self {
            DiagnosticKind::Lex => "Lex",
            DiagnosticKind::Parse => "Parse",
            DiagnosticKind::Semantic => "Semantic",
        }
    }
}

pub fn format_diagnostic(
    kind: DiagnosticKind,
    message: impl AsRef<str>,
    line: Option<u32>,
    col: Option<u32>,
    index: Option<usize>,
) -> String {
    let mut location = String::new();
    if let (Some(line), Some(col)) = (line, col) {
        location.push_str(&format!("line {}, col {}", line, col));
    }
    if let Some(index) = index {
        if !location.is_empty() {
            location.push_str(", ");
        }
        location.push_str(&format!("index {}", index));
    }

    if location.is_empty() {
        format!("{} error: {}", kind.as_str(), message.as_ref())
    } else {
        format!("{} error at {}: {}", kind.as_str(), location, message.as_ref())
    }
}
