use crate::ast_nodes::{Program, ScopeManager, Statement};
use crate::common_types::{Token, TokenKind};
use crate::diagnostics::{DiagnosticKind, format_diagnostic};

mod expressions;
mod statements;

fn parse_statement_at(
    tokens: &[Token],
    current_token_index: usize,
    total_tokens: usize,
    parser_scope: &ScopeManager,
) -> Result<(Statement, usize), String> {
    let start_token = &tokens[current_token_index];

    let parse_result = match start_token.kind() {
        TokenKind::KeywordFn => {
            statements::parse_function_declaration(tokens, current_token_index, parser_scope)
        }
        TokenKind::KeywordFor => {
            statements::parse_for_loop(tokens, current_token_index, parser_scope)
        }
        TokenKind::KeywordWhen => {
            statements::parse_when_statement(tokens, current_token_index, parser_scope)
        }
        TokenKind::KeywordLabel => statements::parse_label_declaration(tokens, current_token_index),
        TokenKind::KeywordStruct => {
            statements::parse_struct_declaration(tokens, current_token_index)
        }
        TokenKind::KeywordOnError => {
            statements::parse_on_block_statement(tokens, current_token_index)
        }
        TokenKind::KeywordIf => statements::parse_if_statement(tokens, current_token_index),
        TokenKind::KeywordWhile => statements::parse_while_statement(tokens, current_token_index),
        TokenKind::KeywordLoop => statements::parse_loop_statement(tokens, current_token_index),
        TokenKind::KeywordBreak | TokenKind::KeywordContinue | TokenKind::KeywordPass => {
            statements::parse_control_keyword_statement(tokens, current_token_index)
        }
        TokenKind::KeywordReturn => statements::parse_return_statement(tokens, current_token_index),
        TokenKind::KeywordNew => statements::parse_new_declaration(tokens, current_token_index),
        TokenKind::Identifier if start_token.lexeme == "iterate" => {
            statements::parse_iterate_loop(tokens, current_token_index, parser_scope)
        }
        TokenKind::Identifier
            if start_token.lexeme == "danger"
                && current_token_index + 1 < total_tokens
                && tokens[current_token_index + 1].kind() == TokenKind::KeywordFn =>
        {
            statements::parse_function_declaration(tokens, current_token_index, parser_scope)
        }
        TokenKind::Identifier => {
            statements::parse_identifier_led_statement(tokens, current_token_index)
        }
        TokenKind::KeywordMy => {
            statements::parse_identifier_led_statement(tokens, current_token_index)
        }
        _ => {
            return Err(format_diagnostic(
                DiagnosticKind::Parse,
                Some("SC-PARSE-001"),
                format!(
                    "unsupported statement start token {:?} ('{}')",
                    start_token.kind(),
                    start_token.lexeme
                ),
                Some(start_token.line),
                Some(start_token.col),
                Some(current_token_index),
            ));
        }
    };

    match parse_result {
        Ok((statement, consumed_count)) if consumed_count > 0 => Ok((statement, consumed_count)),
        Ok(_) => Err(format_diagnostic(
            DiagnosticKind::Parse,
            Some("SC-PARSE-002"),
            "parser consumed zero tokens.",
            Some(start_token.line),
            Some(start_token.col),
            Some(current_token_index),
        )),
        Err(e) => Err(format_diagnostic(
            DiagnosticKind::Parse,
            Some("SC-PARSE-003"),
            e,
            Some(start_token.line),
            Some(start_token.col),
            Some(current_token_index),
        )),
    }
}

pub(super) fn parse_statements_range(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Result<Vec<Statement>, String> {
    let parser_scope = ScopeManager::new(None);
    let mut statements_out = Vec::new();
    let mut current_token_index = start;

    while current_token_index < end {
        let start_token = &tokens[current_token_index];

        if start_token.kind() == TokenKind::NewLine || start_token.kind() == TokenKind::Whitespace {
            current_token_index += 1;
            continue;
        }

        let (statement, consumed_count) =
            parse_statement_at(tokens, current_token_index, end, &parser_scope)?;
        statements_out.push(statement);
        current_token_index += consumed_count;

        while current_token_index < end && tokens[current_token_index].kind() == TokenKind::NewLine
        {
            current_token_index += 1;
        }
    }

    Ok(statements_out)
}

/// Parses a sequence of tokens and constructs a complete Program AST structure.
pub fn parse_program(tokens: &[Token]) -> Result<Program, String> {
    let statements = parse_statements_range(tokens, 0, tokens.len())?;
    Ok(Program { statements })
}
