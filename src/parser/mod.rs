use crate::common_types::{TokenKind, Token};
use crate::ast_nodes::{Program, ScopeManager};
mod statements;

/// Parses a sequence of tokens and constructs a complete Program AST structure.
/// This function orchestrates the process: it reads tokens sequentially 
/// by calling specialized parsers from `parser::statements` and advances the token index based on returned consumption offsets.
pub fn parse_program(tokens: &[Token]) -> Result<Program, String> {
    let mut program = Program::new();
    let parser_scope = ScopeManager::new(None); // Global scope

    let mut current_token_index: usize = 0;
    let total_tokens = tokens.len();

    while current_token_index < total_tokens {
        let start_token = &tokens[current_token_index];

        if start_token.kind() == TokenKind::NewLine || start_token.kind() == TokenKind::Whitespace {
            current_token_index += 1;
            continue;
        }

        let parse_result = match start_token.kind() {
            TokenKind::KeywordFn => statements::parse_function_declaration(tokens, current_token_index, &parser_scope),
            TokenKind::KeywordFor => statements::parse_for_loop(tokens, current_token_index, &parser_scope),
            TokenKind::KeywordWhen => statements::parse_when_statement(tokens, current_token_index, &parser_scope),
            TokenKind::KeywordLabel => statements::parse_label_declaration(tokens, current_token_index),
            TokenKind::KeywordStruct => statements::parse_struct_declaration(tokens, current_token_index),
            TokenKind::KeywordOnError => statements::parse_on_block_statement(tokens, current_token_index),
            TokenKind::KeywordIf => statements::parse_if_statement(tokens, current_token_index),
            TokenKind::KeywordWhile => statements::parse_while_statement(tokens, current_token_index),
            TokenKind::KeywordLoop => statements::parse_loop_statement(tokens, current_token_index),
            TokenKind::Identifier => statements::parse_assignment_statement(tokens, current_token_index),
            _ => {
                return Err(format!(
                    "Unsupported statement start token at index {}: {:?} ('{}')",
                    current_token_index,
                    start_token.kind(),
                    start_token.lexeme
                ));
            }
        };

        match parse_result {
            Ok((statement, consumed_count)) if consumed_count > 0 => {
                program.statements.push(statement);
                current_token_index += consumed_count;
            }
            Ok(_) => {
                return Err(format!(
                    "Parser consumed zero tokens at index {}. Aborting to prevent infinite loop.",
                    current_token_index
                ));
            }
            Err(e) => {
                return Err(format!("Parse error at token index {}: {}", current_token_index, e));
            }
        }

        while current_token_index < total_tokens && tokens[current_token_index].kind() == TokenKind::NewLine {
            current_token_index += 1;
        }
    }

    Ok(program)
}
