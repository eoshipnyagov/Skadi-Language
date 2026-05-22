// File: src/lexer/structures.rs
use crate::common_types::Token;
use crate::diagnostics::{format_diagnostic, DiagnosticKind};

/// Represents an error during lexical analysis.
#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: u32,
    pub col: u32,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            format_diagnostic(
                DiagnosticKind::Lex,
                Some("SC-LEX-001"),
                &self.message,
                Some(self.line),
                Some(self.col),
                None,
            )
        )
    }
}

// Re-exporting Token for use across modules
pub type LexerToken = Token;

/// Utility function to check if a character is likely the start of an operator.
pub fn is_operator_start(c: char) -> bool {
    matches!(c, '{' | '}' | '(' | ')' | '[' | ']' | '.' | ',' | ':' | '+' | '-' | '*' | '/' | '=' | '!' | '>' | '<' | '&' | '|' | '^')
}
