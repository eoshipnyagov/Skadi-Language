// src/parser.rs
use crate::common_types::{TokenKind, Token};
use crate::ast_nodes::{Program, Statement, Expression, ScopeManager, BlockStatement};
use crate::parsing_logic; // <-- NEW IMPORT

/// Parses a sequence of tokens and constructs a complete Program AST structure.
/// This function orchestrates the process: it reads tokens sequentially 
/// by calling specialized parsers from `parsing_logic` and advances the token index based on returned consumption offsets.
pub fn parse_program(tokens: &[Token]) -> Result<Program, String> {
    let mut program = Program::new();
    let parser_scope = ScopeManager::new(None); // Global scope

    // The core logic needs a persistent index pointer into the token stream.
    let mut current_token_index: usize = 0;
    let total_tokens = tokens.len();

    while current_token_index < total_tokens {
        let start_token = &tokens[current_token_index];
        
        println!("-> Attempting to parse statement at token index {}", current_token_index);

        // Parse the first part of a statement according to its type
        if start_token.kind() == TokenKind::KeywordFn {
            match parsing_logic::parse_function_declaration(tokens, current_token_index, &parser_scope) {
                Ok((statement, consumed_count)) => {
                    program.statements.push(statement);
                    current_token_index += consumed_count;
                }
                Err(e) => return Err(format!("Error parsing function at index {}: {}", current_token_index, e)),
            }
        } 
        // Check for Loop/Control Flow (KeywordFor)
        else if start_token.kind() == TokenKind::KeywordFor {
             match parsing_logic::parse_for_loop(tokens, current_token_index, &parser_scope) {
                Ok((statement, consumed_count)) => {
                    program.statements.push(statement);
                    current_token_index += consumed_count;
                }
                Err(e) => return Err(format!("Error parsing for loop at index {}: {}", current_token_index, e)),
            }
        } 
        // Check for While Loop
        else if start_token.kind() == TokenKind::KeywordWhile {
            let mut brace_count = 0;
            
            // Find the opening '{' for the while block (it should be directly after condition)
            let mut check_pos = current_token_index + 1; 
            while check_pos < total_tokens && tokens[check_pos].lexeme != "{" {
                check_pos += 1;
            }
            
            if check_pos < total_tokens && tokens[check_pos].lexeme == "{" {
                // We found the opening brace for the block, now we count braces
                let mut temp_index = check_pos; 
                while temp_index < total_tokens {
                    match tokens[temp_index].kind() {
                        TokenKind::OpPunctuation => {
                            if tokens[temp_index].lexeme == "{" { brace_count += 1; }
                            if tokens[temp_index].lexeme == "}" { brace_count -= 1; }
                        },
                        _ => {}
                    }
                    temp_index += 1;
                    if brace_count <= 0 { break; } // Found matching closing brace
                }
                
                current_token_index = temp_index + 1;
            } else {
                return Err(format!("While statement without opening brace at index {}", current_token_index));
            }
        } 
        // Check for Loop statement
        else if start_token.kind() == TokenKind::KeywordLoop {
             match parsing_logic::parse_for_loop(tokens, current_token_index, &parser_scope) {
                Ok((statement, consumed_count)) => {
                    program.statements.push(statement);
                    current_token_index += consumed_count;
                }
                Err(e) => return Err(format!("Error parsing loop statement at index {}: {}", current_token_index, e)),
            }
        }
        // Check for When Statement 
        else if start_token.kind() == TokenKind::KeywordWhen {
            match parsing_logic::parse_when_statement(tokens, current_token_index, &parser_scope) {
                Ok((statement, consumed_count)) => {
                    program.statements.push(statement);
                    current_token_index += consumed_count;
                }
                Err(e) => return Err(format!("Error parsing when statement at index {}: {}", current_token_index, e)),
            }
        } 
        // Simple Variable Assignment
        else if start_token.kind() == TokenKind::Identifier {
            let target_name = start_token.lexeme.clone();
            let next_index = current_token_index + 1;

            if next_index < total_tokens && tokens[next_index].kind() == TokenKind::OpAssignment {
                // For now, just advance past the identifier and assignment operator
                current_token_index = next_index + 1; 
                
                // We'll create a placeholder statement for assignments
                let placeholder_expr = Box::new(Expression::LiteralInt(0)); // Placeholder
                
                program.statements.push(Statement::Assignment { 
                    target: target_name, 
                    value: placeholder_expr 
                });
            } else {
                // This is an identifier that doesn't start an assignment - skip it
                current_token_index += 1;
            }
        }
        // Handle if statement
        else if start_token.kind() == TokenKind::KeywordIf {
            let mut brace_count = 0;
            
            // Find the opening '{' for the if block 
            let mut check_pos = current_token_index + 1; 
            while check_pos < total_tokens && tokens[check_pos].lexeme != "{" {
                check_pos += 1;
            }
            
            if check_pos < total_tokens && tokens[check_pos].lexeme == "{" {
                // We found the opening brace for the block, now we count braces
                let mut temp_index = check_pos; 
                while temp_index < total_tokens {
                    match tokens[temp_index].kind() {
                        TokenKind::OpPunctuation => {
                            if tokens[temp_index].lexeme == "{" { brace_count += 1; }
                            if tokens[temp_index].lexeme == "}" { brace_count -= 1; }
                        },
                        _ => {}
                    }
                    temp_index += 1;
                    if brace_count <= 0 { break; } // Found matching closing brace
                }
                
                current_token_index = temp_index + 1;
            } else {
                return Err(format!("If statement without opening brace at index {}", current_token_index));
            }
        }
        else {
            // Skip unrecognized tokens for now, but log them
            println!("Warning: Skipping unrecognized token at index {}: {:?}", current_token_index, start_token.kind());
            current_token_index += 1;
        }
    }

    Ok(program)
}
