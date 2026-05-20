// ================================================
// Lexer Utility Functions (Rust)
// File: src/lexer_utils.rs
// ----------------------------------------------------------------

use crate::common_types::TokenKind;

/// Checks if a character is likely to start an operator or punctuation token.
pub fn is_operator_start(c: char) -> bool {
    matches!(c, '{' | '}' | '(' | ')' | '[' | ']' | '.' | ',' | ':' | '+' | '-' | '*' | '/' | '=' | '!' | '>' | '<' | '&' | '|' | '^')
}

/// Helper to resolve keywords (e.g., "fn" -> KeywordFn) vs generic Identifiers ("myFunc").
pub fn resolve_keyword(lexeme: String, kind_resolver: &dyn Fn(&str) -> TokenKind) -> TokenKind {
    // This pattern is complex and highly context-dependent; 
    // for now, we pass a resolver function reference to keep the type correct.
    kind_resolver(&lexeme)
}
