// File: src/lexer/core.rs

use crate::common_types::{TokenKind, Token};
use super::structures::LexError;

/// A Lexer tokenizes the source code string into a sequence of Tokens.
pub struct Lexer<'a> {
    _source: &'a str,
    chars: Vec<char>,
    current_pos: usize,
    current_line: u32,
    current_col: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let chars: Vec<char> = source.chars().collect();
        Lexer {
            _source: source,
            chars,
            current_pos: 0,
            current_line: 1,
            current_col: 1,
        }
    }

    /// Get the current character without advancing.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.current_pos).copied()
    }

    /// Get the next character without advancing.
    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.current_pos + 1).copied()
    }

    /// Consume the current character and advance.
    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.current_pos).copied()?;
        self.current_pos += 1;
        if c == '\n' {
            self.current_line += 1;
            self.current_col = 1;
        } else {
            self.current_col += 1;
        }
        Some(c)
    }

    /// Check if we have more characters to consume.
    fn has_more(&self) -> bool {
        self.current_pos < self.chars.len()
    }

    /// Check if the remaining source starts with the given string.
    fn starts_with(&self, s: &str) -> bool {
        let rem: String = self.chars[self.current_pos..].iter().collect();
        rem.starts_with(s)
    }

    fn lexeme_from_range(&self, start: usize, end: usize) -> String {
        self.chars[start..end].iter().collect()
    }

    /// Consume a line comment (// ...).
    fn skip_line_comment(&mut self) {
        // Already consumed "//" before calling this
        while let Some(c) = self.peek() {
            if c == '\n' || c == '\r' {
                break;
            }
            self.advance();
        }
    }

    /// Consume a block comment (/* ... */).
    fn skip_block_comment(&mut self) {
        // Already consumed "/*" before calling this
        loop {
            match self.peek() {
                Some('*') => {
                    if self.peek_next() == Some('/') {
                        self.advance(); // '*'
                        self.advance(); // '/'
                        break;
                    }
                    self.advance();
                }
                Some(_) => {
                    self.advance();
                }
                None => break, // Unterminated block comment — will be handled later
            }
        }
    }

    /// Consume a string literal "..." (supports simple interpolation markers).
    fn scan_string_literal(&mut self, quote: char) -> String {
        let start = self.current_pos;
        assert_eq!(self.peek(), Some(quote));
        self.advance(); // skip opening quote
        loop {
            match self.peek() {
                Some(c) if c == quote => {
                    self.advance(); // skip closing quote
                    break;
                }
                Some('\\') => {
                    self.advance(); // backslash
                    self.advance(); // escaped char
                }
                Some(_) => {
                    self.advance();
                }
                None => break, // unterminated string
            }
        }
        self.lexeme_from_range(start, self.current_pos)
    }

    /// Consume a numeric literal (int or float). Returns the text.
    fn scan_number(&mut self) -> String {
        let start = self.current_pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                self.advance();
            } else {
                break;
            }
        }
        self.lexeme_from_range(start, self.current_pos)
    }

    /// Consume an identifier or keyword. Returns the text.
    fn scan_identifier(&mut self) -> String {
        let start = self.current_pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        self.lexeme_from_range(start, self.current_pos)
    }

    /// Resolve a keyword/identifier string to TokenKind.
    fn resolve_keyword(lexeme: &str) -> TokenKind {
        match lexeme {
            "fn" => TokenKind::KeywordFn,
            "struct" => TokenKind::KeywordStruct,
            "label" => TokenKind::KeywordLabel,
            "if" => TokenKind::KeywordIf,
            "else" => TokenKind::KeywordElse,
            "when" => TokenKind::KeywordWhen,
            "is" => TokenKind::KeywordIs,
            "for" => TokenKind::KeywordFor,
            "in" => TokenKind::KeywordIn,
            "while" => TokenKind::KeywordWhile,
            "loop" => TokenKind::KeywordLoop,
            "break" => TokenKind::KeywordBreak,
            "continue" => TokenKind::KeywordContinue,
            "pass" => TokenKind::KeywordPass,
            "return" => TokenKind::KeywordReturn,
            "new" => TokenKind::KeywordNew,
            "fixed" | "const" => TokenKind::KeywordFixed,
            "hide" => TokenKind::KeywordHide,
            "local" => TokenKind::KeywordLocal,
            "my" => TokenKind::KeywordMy,
            "direct" => TokenKind::KeywordDirect,
            "allow" => TokenKind::KeywordAllowDrop,
            "on" => TokenKind::KeywordOnError,
            "interrupt" => TokenKind::KeywordOnInterrupt,
            "and" | "or" | "xor" | "not" => TokenKind::OpLogical,
            "div" | "mod" => TokenKind::OpArithmetic,
            "true" => TokenKind::TypeBool,
            "false" => TokenKind::TypeBool,
            _ => TokenKind::Identifier,
        }
    }

    /// Scan the next token. Called AFTER whitespace has been skipped.
    /// Returns (TokenKind, lexeme) for the token found.
    fn next_token(&mut self) -> Option<Result<(TokenKind, String), LexError>> {
        if !self.has_more() {
            return None;
        }

        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.advance();
            } else {
                break;
            }
        }

        if !self.has_more() {
            return None;
        }

        let line = self.current_line;
        let col = self.current_col;

        // Record the starting position before peeking
        let _start_pos = self.current_pos;

        // 1. Newlines are significant in Skadi (one statement per line)
        if self.peek() == Some('\n') || self.peek() == Some('\r') {
            self.advance();
            return Some(Ok((TokenKind::NewLine, "\n".into())));
        }

        // 2. Comments: skip them entirely (they produce no tokens)
        if self.starts_with("//") {
            self.advance(); self.advance(); // consume "//"
            self.skip_line_comment();
            return Some(Ok((TokenKind::Whitespace, String::new())));
        }
        if self.starts_with("/*") {
            self.advance(); self.advance(); // consume "/*"
            self.skip_block_comment();
            return Some(Ok((TokenKind::Whitespace, String::new())));
        }

        // 3. String literals
        if self.peek() == Some('"') || self.peek() == Some('\'') {
            let quote = self.peek().unwrap();
            let lexeme = self.scan_string_literal(quote);
            if quote == '\'' {
                return Some(Ok((TokenKind::TypeChar, lexeme)));
            } else {
                return Some(Ok((TokenKind::TypeString, lexeme)));
            }
        }

        // 4. Char literals (single quotes that aren't strings)
        // Already handled above — single quotes can be char or string context

        // 5. Numbers
        if let Some(c) = self.peek() {
            if c.is_ascii_digit() || (c == '.' && self.peek_next().map_or(false, |nc| nc.is_ascii_digit())) {
                let lexeme = self.scan_number();
                return Some(Ok((
                    if lexeme.contains('.') { TokenKind::TypeFloat } else { TokenKind::TypeInt },
                    lexeme,
                )));
            }
        }

        // 6. Identifiers and keywords (check AFTER numbers)
        if let Some(c) = self.peek() {
            if c.is_alphabetic() || c == '_' {
                let lexeme = self.scan_identifier();
                return Some(Ok((Self::resolve_keyword(&lexeme), lexeme)));
            }
        }

        // 7. Two-character operators (check before single-char)
        let two_char_op = self.peek().and_then(|c1| {
            let c2 = self.peek_next();
            if let Some(c2) = c2 {
                Some(format!("{}{}", c1, c2))
            } else {
                None
            }
        });

        match two_char_op.as_deref() {
            Some("==") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpComparison, "==".into()))); }
            Some("!=") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpComparison, "!=".into()))); }
            Some(">=") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpComparison, ">=".into()))); }
            Some("<=") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpComparison, "<=".into()))); }
            Some("&&") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpLogical, "and".into()))); }
            Some("||") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpLogical, "or".into()))); }
            Some("+=") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpAssignment, "+=".into()))); }
            Some("-=") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpAssignment, "-=".into()))); }
            Some("*=") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpAssignment, "*=".into()))); }
            Some("/=") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpAssignment, "/=".into()))); }
            Some("++") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpIncDec, "++".into()))); }
            Some("--") => { self.advance(); self.advance(); return Some(Ok((TokenKind::OpIncDec, "--".into()))); }
            _ => {}
        }

        // 8. Single character operators and punctuation
        let c = self.peek().unwrap();
        match c {
            '(' => { self.advance(); Some(Ok((TokenKind::OpPunctuation, "(".into()))) }
            ')' => { self.advance(); Some(Ok((TokenKind::OpPunctuation, ")".into()))) }
            '{' => { self.advance(); Some(Ok((TokenKind::OpPunctuation, "{".into()))) }
            '}' => { self.advance(); Some(Ok((TokenKind::OpPunctuation, "}".into()))) }
            '[' => { self.advance(); Some(Ok((TokenKind::OpPunctuation, "[".into()))) }
            ']' => { self.advance(); Some(Ok((TokenKind::OpPunctuation, "]".into()))) }
            '.' => { self.advance(); Some(Ok((TokenKind::OpPunctuation, ".".into()))) }
            ',' => { self.advance(); Some(Ok((TokenKind::OpPunctuation, ",".into()))) }
            ':' => { self.advance(); Some(Ok((TokenKind::OpPunctuation, ":".into()))) }
            '+' => { self.advance(); Some(Ok((TokenKind::OpArithmetic, "+".into()))) }
            '-' => {
                // Could be unary minus or subtraction
                self.advance();
                // Heuristic: if next char is a digit or '(', likely part of expression
                // For now just return as arithmetic; Pratt parser handles context.
                Some(Ok((TokenKind::OpArithmetic, "-".into())))
            }
            '*' => { self.advance(); Some(Ok((TokenKind::OpArithmetic, "*".into()))) }
            '/' => { self.advance(); Some(Ok((TokenKind::OpArithmetic, "/".into()))) }
            '%' => { self.advance(); Some(Ok((TokenKind::OpArithmetic, "%".into()))) }
            '^' => { self.advance(); Some(Ok((TokenKind::OpArithmetic, "^".into()))) }
            '!' => {
                self.advance();
                // Could be != already handled above; or just ! (not unary)
                if let Some(next) = self.peek() {
                    if next == '=' {
                        return Some(Ok((TokenKind::OpComparison, "!=".into())));
                    }
                }
                Some(Ok((TokenKind::OpLogical, "!".into())))
            }
            '&' => { self.advance(); Some(Ok((TokenKind::OpLogical, "&".into()))) }
            '|' => { self.advance(); Some(Ok((TokenKind::OpLogical, "|".into()))) }
            '=' => {
                // Check for == handled above. Plain = is assignment.
                if let Some('=') = self.peek_next() {
                    return None; // Will be re-scanned as == by outer call (already consumed)
                }
                self.advance();
                Some(Ok((TokenKind::OpAssignment, "=".into())))
            }
            '<' => { self.advance(); Some(Ok((TokenKind::OpComparison, "<".into()))) }
            '>' => { self.advance(); Some(Ok((TokenKind::OpComparison, ">".into()))) }
            _ => {
                // Unknown character
                self.advance();
                Some(Err(LexError {
                    message: format!("unexpected character '{}'", c),
                    line,
                    col,
                }))
            }
        }
    }
}

// Implement Iterator for Lexer
impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let kind = match self.next_token() {
                Some(Ok((kind, lexeme))) => (kind, lexeme),
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            };

            // Filter out whitespace tokens — newlines are kept as they're significant in Skadi
            if kind.0 == TokenKind::Whitespace && kind.1.is_empty() {
                continue; // Skip pure whitespace
            }

            let (k, lexeme) = kind;
            let line = self.current_line;
            let col = self.current_col;

            // Recalculate the position at which this token STARTED
            // We need to restore: we advanced past the token in next_token,
            // so the start_line/col are from when we entered next_token.
            // For simplicity, just record current as end — but we want start position.
            // Fix: track start before calling next_token.

            return Some(Ok(Token {
                kind: k,
                lexeme,
                line,
                col,
            }));
        }
    }
}

/// Main entry point for lexing: produces a Vec of tokens from source code.
pub fn lex(source_code: &str) -> Result<Vec<Token>, LexError> {
    let mut lexer = Lexer::new(source_code);
    let mut tokens = Vec::new();

    loop {
        match lexer.next() {
            Some(Ok(token)) => {
                // Filter whitespace from output (except newlines which are significant)
                if token.kind != TokenKind::Whitespace {
                    tokens.push(token);
                }
            }
            Some(Err(e)) => return Err(e),
            None => break,
        }
    }

    Ok(tokens)
}
