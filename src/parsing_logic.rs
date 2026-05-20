// ================================================
// Parser Logic Helpers (Rust)
// File: src/parsing_logic.rs
// ----------------------------------------------------------------
use crate::common_types::{Token, TokenKind};
use crate::ast_nodes::{Statement, Expression, ScopeManager, BlockStatement};

/// Core type for a parser function: consumes tokens and returns the resulting AST node and the count of consumed tokens.
pub type ParseResult<T> = Result<(T, usize), String>;


/// Parses a full function definition (fn name(...) { ... }) from a token stream.
/// Consumes all necessary tokens from 'tokens' starting *after* the initial 'fn' keyword.
pub fn parse_function_declaration(
    tokens: &[Token], 
    start_index: usize, 
    scope: &ScopeManager
) -> ParseResult<Statement> {
    let mut current_index = start_index;
    
    // Minimum checks required to prevent panics on incomplete input.
    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordFn {
        return Err("Expected 'fn' keyword but found nothing.".into());
    }

    // 1. Consume "fn" (already passed start_index, so we expect the next token)
    let function_name = if current_index + 1 < tokens.len() && tokens[current_index+1].kind() == TokenKind::Identifier {
        tokens[current_index+1].lexeme.clone()
    } else {
         return Err("Function definition must be followed by an identifier (function name).".into());
    };

    // 2. Consume '(' and parse parameter list
    if current_index + 2 >= tokens.len() || tokens[current_index+2].kind() != TokenKind::OpPunctuation {
        return Err("Function signature expected '('.".into());
    }
    
    // Parse parameters
    let mut params = Vec::new();
    current_index += 3; // Move past "fn name ("
    
    if current_index < tokens.len() && tokens[current_index].kind() == TokenKind::OpPunctuation && tokens[current_index].lexeme == ")" {
        // No parameters
        current_index += 1;
    } else {
        // Parse parameter list - very simplified implementation
        while current_index < tokens.len() && tokens[current_index].kind() != TokenKind::OpPunctuation && tokens[current_index].lexeme != ")" {
            if tokens[current_index].kind() == TokenKind::Identifier {
                params.push(tokens[current_index].lexeme.clone());
            }
            current_index += 1;
        }
        // Skip the closing ')'
        if current_index < tokens.len() && tokens[current_index].lexeme == ")" {
            current_index += 1;
        } else {
            return Err("Function signature expected ')' after parameters.".into());
        }
    }

    // 3. Parse return type or nothing (for void functions)
    let returns = None; // Simplified for now - we don't parse complex return types yet
    
    // 4. Expecting '{' to begin the body block
    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::OpPunctuation || tokens[current_index].lexeme != "{" {
        return Err("Function signature expected '{{' to begin the body block.".into());
    }
    
    // 5. Parse function body
    let start_body = current_index;
    // Find matching closing brace (this is a very basic approach)
    let mut brace_count = 1; 
    current_index += 1; 
    while current_index < tokens.len() && brace_count > 0 {
        match tokens[current_index].kind() {
            TokenKind::OpPunctuation => {
                if tokens[current_index].lexeme == "{" { brace_count += 1; }
                if tokens[current_index].lexeme == "}" { brace_count -= 1; }
            },
            _ => {}
        }
        current_index += 1;
    }
    
    // Create a placeholder body for now
    let stmt = Statement::FunctionDef { 
        name: function_name, 
        params, 
        body: BlockStatement { statements: vec![] }.into(),
        returns,
        is_danger: false
    };

    Ok((stmt, current_index - start_index))
}


/// Parses a 'for' loop structure. Consumes tokens for initialization, condition, and update blocks.
pub fn parse_for_loop(
    tokens: &[Token], 
    start_index: usize, 
    scope: &ScopeManager
) -> ParseResult<Statement> {
    // Expected pattern: for (init; condition; update) { ... }
    let mut current_index = start_index;
    
    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordFor {
        return Err("Expected 'for' keyword to start loop.".into());
    }
    
    // Skip the 'for'
    current_index += 1;
    
    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::OpPunctuation || tokens[current_index].lexeme != "(" {
        return Err("For loop expected '(' after keyword.".into());
    }
    
    // Skip the opening parenthesis
    current_index += 1;
    
    // Parse initialization expression (if exists)
    if current_index < tokens.len() && tokens[current_index].lexeme != ";" {
        // For now we don't actually parse expressions, just skip to semicolon
        while current_index < tokens.len() && tokens[current_index].lexeme != ";" {
            current_index += 1;
        }
    }
    
    if current_index >= tokens.len() || tokens[current_index].lexeme != ";" {
        return Err("For loop expected ';' after initialization.".into());
    }
    
    // Skip the semicolon
    current_index += 1;
    
    // Parse condition expression (if exists)
    if current_index < tokens.len() && tokens[current_index].lexeme != ";" {
        // For now we don't actually parse expressions, just skip to semicolon
        while current_index < tokens.len() && tokens[current_index].lexeme != ";" {
            current_index += 1;
        }
    }
    
    if current_index >= tokens.len() || tokens[current_index].lexeme != ";" {
        return Err("For loop expected ';' after condition.".into());
    }
    
    // Skip the semicolon
    current_index += 1;
    
    // Parse update expression (if exists)
    if current_index < tokens.len() && tokens[current_index].lexeme != "}" {
        // For now we don't actually parse expressions, just skip to closing brace
        while current_index < tokens.len() && tokens[current_index].lexeme != "}" {
            current_index += 1;
        }
    }
    
    if current_index >= tokens.len() || tokens[current_index].lexeme != "{" {
        return Err("For loop expected '{' to begin body.".into());
    }
    
    // Parse the loop body - just find matching closing brace
    let start_body = current_index;
    let mut brace_count = 1; 
    current_index += 1; 
    while current_index < tokens.len() && brace_count > 0 {
        match tokens[current_index].kind() {
            TokenKind::OpPunctuation => {
                if tokens[current_index].lexeme == "{" { brace_count += 1; }
                if tokens[current_index].lexeme == "}" { brace_count -= 1; }
            },
            _ => {}
        }
        current_index += 1;
    }
    
    let stmt = Statement::ForLoop {
        initialization: None,
        condition: None, 
        update: None, 
        body: BlockStatement { statements: vec![] }.into()
    };
    
    Ok((stmt, current_index - start_index))
}

/// Parses a 'when' statement. Handles 'when' keyword followed by an expression and a body block.
pub fn parse_when_statement(
    tokens: &[Token], 
    start_index: usize, 
    scope: &ScopeManager
) -> ParseResult<Statement> {
    let mut current_index = start_index;
    
    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordWhen {
        return Err("Expected 'when' keyword to start statement.".into());
    }
    
    // Skip the 'when'
    current_index += 1;
    
    // Parse when expression (we'll just skip for now)
    while current_index < tokens.len() && tokens[current_index].lexeme != "{" {
        current_index += 1;
    }
    
    if current_index >= tokens.len() || tokens[current_index].lexeme != "{" {
        return Err("When statement expected '{' to begin body.".into());
    }
    
    // Parse the when block - find matching closing brace
    let mut brace_count = 1; 
    current_index += 1; 
    while current_index < tokens.len() && brace_count > 0 {
        match tokens[current_index].kind() {
            TokenKind::OpPunctuation => {
                if tokens[current_index].lexeme == "{" { brace_count += 1; }
                if tokens[current_index].lexeme == "}" { brace_count -= 1; }
            },
            _ => {}
        }
        current_index += 1;
    }
    
    let stmt = Statement::WhenBlock {
        when_expression: Box::new(Expression::LiteralInt(0)), // Placeholder
        cases: vec![],
        else_block: None
    };
    
    Ok((stmt, current_index - start_index))
}
