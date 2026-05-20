// ================================================
// Parser Logic Helpers (Rust)
// File: src/parser/statements.rs
// ----------------------------------------------------------------
use crate::common_types::{Token, TokenKind};
use crate::ast_nodes::{Statement, ScopeManager, BlockStatement};
use super::expressions::parse_expression_range;

/// Core type for a parser function: consumes tokens and returns the resulting AST node and the count of consumed tokens.
pub type ParseResult<T> = Result<(T, usize), String>;

fn find_block_end(tokens: &[Token], open_brace_index: usize) -> Result<usize, String> {
    if open_brace_index >= tokens.len() || tokens[open_brace_index].lexeme != "{" {
        return Err("Expected '{'.".into());
    }
    let mut brace_count = 1usize;
    let mut current = open_brace_index + 1;
    while current < tokens.len() && brace_count > 0 {
        if tokens[current].kind() == TokenKind::OpPunctuation {
            if tokens[current].lexeme == "{" {
                brace_count += 1;
            } else if tokens[current].lexeme == "}" {
                brace_count -= 1;
            }
        }
        current += 1;
    }
    if brace_count != 0 {
        return Err("Unterminated block: missing '}'.".into());
    }
    Ok(current - 1)
}


/// Parses a full function definition (fn name(...) { ... }) from a token stream.
/// Consumes all necessary tokens from 'tokens' starting *after* the initial 'fn' keyword.
pub fn parse_function_declaration(
    tokens: &[Token], 
    start_index: usize, 
    _scope: &ScopeManager
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
    if current_index + 2 >= tokens.len()
        || tokens[current_index + 2].kind() != TokenKind::OpPunctuation
        || tokens[current_index + 2].lexeme != "("
    {
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
        while current_index < tokens.len() && tokens[current_index].lexeme != ")" {
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
    let block_end = find_block_end(tokens, current_index)?;
    current_index = block_end + 1;
    
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
    _scope: &ScopeManager
) -> ParseResult<Statement> {
    // Supports both:
    // 1) Skadi style: for item in collection { ... }
    // 2) C-style: for (init; condition; update) { ... } (legacy scaffold path)
    let mut current_index = start_index;
    
    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordFor {
        return Err("Expected 'for' keyword to start loop.".into());
    }
    
    // Skip "for"
    current_index += 1;

    // Skadi for-in path
    if current_index < tokens.len() && tokens[current_index].kind() == TokenKind::Identifier {
        let loop_var = tokens[current_index].lexeme.clone();
        current_index += 1;
        if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordIn {
            return Err("For loop expected 'in' after iterator variable.".into());
        }
        current_index += 1;
        let expr_start = current_index;
        while current_index < tokens.len() && tokens[current_index].lexeme != "{" {
            current_index += 1;
        }
        if current_index >= tokens.len() {
            return Err("For-in loop expected '{' to begin body.".into());
        }
        let collection_expr = parse_expression_range(tokens, expr_start, current_index)?;
        let block_end = find_block_end(tokens, current_index)?;
        current_index = block_end + 1;
        let stmt = Statement::ForLoop {
            initialization: Some(Box::new(crate::ast_nodes::Expression::VariableReference(loop_var))),
            condition: Some(Box::new(collection_expr)),
            update: None,
            body: Box::new(BlockStatement { statements: vec![] }),
        };
        return Ok((stmt, current_index - start_index));
    }

    // Legacy C-style fallback
    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::OpPunctuation || tokens[current_index].lexeme != "(" {
        return Err("For loop expected iterator variable or '(' after keyword.".into());
    }
    current_index += 1;
    while current_index < tokens.len() && tokens[current_index].lexeme != ";" {
        current_index += 1;
    }
    if current_index >= tokens.len() || tokens[current_index].lexeme != ";" {
        return Err("For loop expected ';' after initialization.".into());
    }
    current_index += 1;
    while current_index < tokens.len() && tokens[current_index].lexeme != ";" {
        current_index += 1;
    }
    if current_index >= tokens.len() || tokens[current_index].lexeme != ";" {
        return Err("For loop expected ';' after condition.".into());
    }
    current_index += 1;
    while current_index < tokens.len() && tokens[current_index].lexeme != ")" {
        current_index += 1;
    }
    if current_index >= tokens.len() || tokens[current_index].lexeme != ")" {
        return Err("For loop expected ')'.".into());
    }
    current_index += 1;
    if current_index >= tokens.len() || tokens[current_index].lexeme != "{" {
        return Err("For loop expected '{' to begin body.".into());
    }
    let block_end = find_block_end(tokens, current_index)?;
    current_index = block_end + 1;
    Ok((
        Statement::ForLoop {
            initialization: None,
            condition: None,
            update: None,
            body: Box::new(BlockStatement { statements: vec![] }),
        },
        current_index - start_index,
    ))
}

/// Parses a 'when' statement. Handles 'when' keyword followed by an expression and a body block.
pub fn parse_when_statement(
    tokens: &[Token], 
    start_index: usize, 
    _scope: &ScopeManager
) -> ParseResult<Statement> {
    let mut current_index = start_index;
    
    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordWhen {
        return Err("Expected 'when' keyword to start statement.".into());
    }
    
    // Skip the 'when'
    current_index += 1;
    
    let expr_start = current_index;
    while current_index < tokens.len() && tokens[current_index].lexeme != "{" {
        current_index += 1;
    }
    
    if current_index >= tokens.len() || tokens[current_index].lexeme != "{" {
        return Err("When statement expected '{' to begin body.".into());
    }
    
    let expr_end = current_index;
    // Parse the when block - find matching closing brace
    let block_end = find_block_end(tokens, current_index)?;
    current_index = block_end + 1;
    
    let when_expression = parse_expression_range(tokens, expr_start, expr_end)?;
    let stmt = Statement::WhenBlock {
        when_expression: Box::new(when_expression),
        cases: vec![],
        else_block: None
    };
    
    Ok((stmt, current_index - start_index))
}

pub fn parse_if_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordIf {
        return Err("Expected 'if' keyword.".into());
    }
    let expr_start = start_index + 1;
    let mut cursor = expr_start;
    while cursor < tokens.len() && tokens[cursor].lexeme != "{" {
        cursor += 1;
    }
    if cursor >= tokens.len() {
        return Err("If statement expected '{'.".into());
    }
    let then_end = find_block_end(tokens, cursor)?;
    let mut consumed_to = then_end + 1;
    let mut else_block = None;

    if consumed_to < tokens.len() && tokens[consumed_to].kind() == TokenKind::KeywordElse {
        consumed_to += 1;
        if consumed_to < tokens.len() && tokens[consumed_to].kind() == TokenKind::KeywordIf {
            let (_, else_if_consumed) = parse_if_statement(tokens, consumed_to)?;
            consumed_to += else_if_consumed;
            else_block = Some(Box::new(BlockStatement { statements: vec![] }));
        } else {
            if consumed_to >= tokens.len() || tokens[consumed_to].lexeme != "{" {
                return Err("Else branch expected '{'.".into());
            }
            let else_end = find_block_end(tokens, consumed_to)?;
            consumed_to = else_end + 1;
            else_block = Some(Box::new(BlockStatement { statements: vec![] }));
        }
    }

    let condition = parse_expression_range(tokens, expr_start, cursor)?;
    Ok((
        Statement::IfStatement {
            condition: Box::new(condition),
            then_block: Box::new(BlockStatement { statements: vec![] }),
            else_block,
        },
        consumed_to - start_index,
    ))
}

pub fn parse_while_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordWhile {
        return Err("Expected 'while' keyword.".into());
    }
    let expr_start = start_index + 1;
    let mut cursor = expr_start;
    while cursor < tokens.len() && tokens[cursor].lexeme != "{" {
        cursor += 1;
    }
    if cursor >= tokens.len() {
        return Err("While statement expected '{'.".into());
    }
    let block_end = find_block_end(tokens, cursor)?;
    let condition = parse_expression_range(tokens, expr_start, cursor)?;
    Ok((
        Statement::WhileLoop {
            condition: Box::new(condition),
            body: Box::new(BlockStatement { statements: vec![] }),
        },
        block_end + 1 - start_index,
    ))
}

pub fn parse_loop_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordLoop {
        return Err("Expected 'loop' keyword.".into());
    }
    let open = start_index + 1;
    if open >= tokens.len() || tokens[open].lexeme != "{" {
        return Err("Loop statement expected '{' after 'loop'.".into());
    }
    let block_end = find_block_end(tokens, open)?;
    Ok((
        Statement::LoopStatement {
            body: Box::new(BlockStatement { statements: vec![] }),
        },
        block_end + 1 - start_index,
    ))
}

pub fn parse_assignment_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if start_index + 1 >= tokens.len() {
        return Err("Incomplete assignment.".into());
    }
    if tokens[start_index].kind() != TokenKind::Identifier {
        return Err("Assignment must start with identifier.".into());
    }
    if tokens[start_index + 1].kind() != TokenKind::OpAssignment {
        return Err("Expected assignment operator.".into());
    }
    let target_name = tokens[start_index].lexeme.clone();
    let mut cursor = start_index + 2;
    while cursor < tokens.len() {
        if tokens[cursor].kind() == TokenKind::NewLine || tokens[cursor].lexeme == "}" {
            break;
        }
        cursor += 1;
    }
    let value = parse_expression_range(tokens, start_index + 2, cursor)?;
    Ok((
        Statement::Assignment {
            target: target_name,
            value: Box::new(value),
        },
        (cursor - start_index).max(2),
    ))
}

pub fn parse_label_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordLabel {
        return Err("Expected 'label' keyword.".into());
    }
    if start_index + 1 >= tokens.len() || tokens[start_index + 1].kind() != TokenKind::Identifier {
        return Err("Label declaration expected identifier name.".into());
    }
    let open = start_index + 2;
    if open >= tokens.len() || tokens[open].lexeme != "{" {
        return Err("Label declaration expected '{'.".into());
    }
    let mut variants = Vec::new();
    let mut cursor = open + 1;
    while cursor < tokens.len() && tokens[cursor].lexeme != "}" {
        if tokens[cursor].kind() == TokenKind::Identifier {
            variants.push(tokens[cursor].lexeme.clone());
        }
        cursor += 1;
    }
    let close = find_block_end(tokens, open)?;
    Ok((
        Statement::LabelDecl {
            name: tokens[start_index + 1].lexeme.clone(),
            variants,
        },
        close + 1 - start_index,
    ))
}

pub fn parse_struct_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordStruct {
        return Err("Expected 'struct' keyword.".into());
    }
    if start_index + 1 >= tokens.len() || tokens[start_index + 1].kind() != TokenKind::Identifier {
        return Err("Struct declaration expected identifier name.".into());
    }
    let open = start_index + 2;
    if open >= tokens.len() || tokens[open].lexeme != "{" {
        return Err("Struct declaration expected '{'.".into());
    }
    let close = find_block_end(tokens, open)?;
    Ok((
        Statement::StructDecl {
            name: tokens[start_index + 1].lexeme.clone(),
        },
        close + 1 - start_index,
    ))
}

pub fn parse_on_block_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordOnError {
        return Err("Expected 'on' keyword.".into());
    }
    let mut cursor = start_index + 1;
    while cursor < tokens.len() && tokens[cursor].lexeme != "{" {
        cursor += 1;
    }
    if cursor >= tokens.len() {
        return Err("on-block expected '{'.".into());
    }
    let close = find_block_end(tokens, cursor)?;
    let trigger = if start_index + 1 < tokens.len() {
        tokens[start_index + 1].lexeme.clone()
    } else {
        "unknown".to_string()
    };
    Ok((
        Statement::OnBlock { trigger },
        close + 1 - start_index,
    ))
}
