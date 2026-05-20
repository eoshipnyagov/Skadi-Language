// src/parser.rs
use crate::common_types::{TokenKind, Token};
use crate::ast_nodes::{Program, ScopeManager};
use crate::parsing_logic;

/// Parses a sequence of tokens and constructs a complete Program AST structure.
/// This function orchestrates the process: it reads tokens sequentially 
/// by calling specialized parsers from `parsing_logic` and advances the token index based on returned consumption offsets.
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
            TokenKind::KeywordFn => parsing_logic::parse_function_declaration(tokens, current_token_index, &parser_scope),
            TokenKind::KeywordFor => parsing_logic::parse_for_loop(tokens, current_token_index, &parser_scope),
            TokenKind::KeywordWhen => parsing_logic::parse_when_statement(tokens, current_token_index, &parser_scope),
            TokenKind::KeywordLabel => parsing_logic::parse_label_declaration(tokens, current_token_index),
            TokenKind::KeywordStruct => parsing_logic::parse_struct_declaration(tokens, current_token_index),
            TokenKind::KeywordOnError => parsing_logic::parse_on_block_statement(tokens, current_token_index),
            TokenKind::KeywordIf => parsing_logic::parse_if_statement(tokens, current_token_index),
            TokenKind::KeywordWhile => parsing_logic::parse_while_statement(tokens, current_token_index),
            TokenKind::KeywordLoop => parsing_logic::parse_loop_statement(tokens, current_token_index),
            TokenKind::Identifier => parsing_logic::parse_assignment_statement(tokens, current_token_index),
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

#[cfg(test)]
mod tests {
    use super::parse_program;
    use crate::ast_nodes::Statement;
    use crate::lexer::core::lex;

    #[test]
    fn parses_label_and_struct_top_level() {
        let src = r#"
label Status {
    Ok
    Error
}

struct Sensor {
    u8 address
}
"#;
        let tokens = lex(src).expect("lex should succeed");
        let program = parse_program(&tokens).expect("parse should succeed");
        assert_eq!(program.statements.len(), 2);
        match &program.statements[0] {
            Statement::LabelDecl { name, variants } => {
                assert_eq!(name, "Status");
                assert!(variants.contains(&"Ok".to_string()));
                assert!(variants.contains(&"Error".to_string()));
            }
            _ => panic!("expected LabelDecl"),
        }
        match &program.statements[1] {
            Statement::StructDecl { name } => assert_eq!(name, "Sensor"),
            _ => panic!("expected StructDecl"),
        }
    }

    #[test]
    fn parses_on_interrupt_block() {
        let src = r#"
on interrupt timer0 {
    output("tick")
}
"#;
        let tokens = lex(src).expect("lex should succeed");
        let program = parse_program(&tokens).expect("parse should succeed");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::OnBlock { trigger } => assert_eq!(trigger, "interrupt"),
            _ => panic!("expected OnBlock"),
        }
    }
}
