// ================================================
// Parser Logic Helpers (Rust)
// File: src/parser/statements.rs
// ----------------------------------------------------------------
use crate::ast_nodes::{BlockStatement, FunctionParam, ScopeManager, Statement};
use crate::common_types::{Token, TokenKind};

use super::expressions::parse_expression_range;
use super::parse_statements_range;

/// Core type for a parser function: consumes tokens and returns the resulting AST node and the count of consumed tokens.
pub type ParseResult<T> = Result<(T, usize), String>;

fn parse_expression_list(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Result<Vec<crate::ast_nodes::Expression>, String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut seg_start = start;
    let mut i = start;
    while i < end {
        let t = &tokens[i];
        if t.lexeme == "(" {
            depth += 1;
        } else if t.lexeme == ")" {
            depth = depth.saturating_sub(1);
        } else if t.lexeme == "," && depth == 0 {
            if seg_start < i {
                out.push(parse_expression_range(tokens, seg_start, i)?);
            }
            seg_start = i + 1;
        }
        i += 1;
    }
    if seg_start < end {
        out.push(parse_expression_range(tokens, seg_start, end)?);
    }
    Ok(out)
}

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

pub fn parse_function_declaration(
    tokens: &[Token],
    start_index: usize,
    _scope: &ScopeManager,
) -> ParseResult<Statement> {
    let mut current_index = start_index;
    let mut is_danger = false;

    if current_index < tokens.len()
        && tokens[current_index].kind() == TokenKind::Identifier
        && tokens[current_index].lexeme == "danger"
    {
        is_danger = true;
        current_index += 1;
    }

    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordFn {
        return Err("Expected 'fn' keyword but found nothing.".into());
    }

    let function_name = if current_index + 1 < tokens.len()
        && tokens[current_index + 1].kind() == TokenKind::Identifier
    {
        tokens[current_index + 1].lexeme.clone()
    } else {
        return Err("Function definition must be followed by an identifier (function name).".into());
    };

    if current_index + 2 >= tokens.len()
        || tokens[current_index + 2].kind() != TokenKind::OpPunctuation
        || tokens[current_index + 2].lexeme != "("
    {
        return Err("Function signature expected '('.".into());
    }

    let mut params = Vec::new();
    current_index += 3;

    if current_index < tokens.len()
        && tokens[current_index].kind() == TokenKind::OpPunctuation
        && tokens[current_index].lexeme == ")"
    {
        current_index += 1;
    } else {
        while current_index < tokens.len() && tokens[current_index].lexeme != ")" {
            if tokens[current_index].kind() == TokenKind::OpPunctuation
                && tokens[current_index].lexeme == ","
            {
                current_index += 1;
                continue;
            }

            if current_index + 1 < tokens.len()
                && tokens[current_index].kind() == TokenKind::Identifier
                && tokens[current_index + 1].kind() == TokenKind::Identifier
            {
                params.push(FunctionParam {
                    param_type: Some(tokens[current_index].lexeme.clone()),
                    name: tokens[current_index + 1].lexeme.clone(),
                });
                current_index += 2;
                continue;
            }

            if tokens[current_index].kind() == TokenKind::Identifier {
                params.push(FunctionParam {
                    param_type: None,
                    name: tokens[current_index].lexeme.clone(),
                });
            }
            current_index += 1;
        }
        if current_index < tokens.len() && tokens[current_index].lexeme == ")" {
            current_index += 1;
        } else {
            return Err("Function signature expected ')' after parameters.".into());
        }
    }

    let mut returns = None;
    if current_index < tokens.len()
        && tokens[current_index].kind() == TokenKind::Identifier
        && current_index + 1 < tokens.len()
        && tokens[current_index + 1].lexeme == "{"
    {
        returns = Some(tokens[current_index].lexeme.clone());
        current_index += 1;
    }

    if current_index >= tokens.len()
        || tokens[current_index].kind() != TokenKind::OpPunctuation
        || tokens[current_index].lexeme != "{"
    {
        return Err("Function signature expected '{' to begin the body block.".into());
    }

    let open_brace = current_index;
    let block_end = find_block_end(tokens, open_brace)?;
    let inner_statements = parse_statements_range(tokens, open_brace + 1, block_end)?;
    current_index = block_end + 1;

    let stmt = Statement::FunctionDef {
        name: function_name,
        params,
        body: BlockStatement {
            statements: inner_statements,
        }
        .into(),
        returns,
        is_danger,
    };

    Ok((stmt, current_index - start_index))
}

pub fn parse_for_loop(tokens: &[Token], start_index: usize, _scope: &ScopeManager) -> ParseResult<Statement> {
    let mut current_index = start_index;

    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordFor {
        return Err("Expected 'for' keyword to start loop.".into());
    }

    current_index += 1;

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
        let body_statements = parse_statements_range(tokens, current_index + 1, block_end)?;
        current_index = block_end + 1;
        let stmt = Statement::ForLoop {
            initialization: Some(Box::new(crate::ast_nodes::Expression::VariableReference(loop_var))),
            condition: Some(Box::new(collection_expr)),
            update: None,
            body: Box::new(BlockStatement { statements: body_statements }),
        };
        return Ok((stmt, current_index - start_index));
    }

    if current_index >= tokens.len()
        || tokens[current_index].kind() != TokenKind::OpPunctuation
        || tokens[current_index].lexeme != "("
    {
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
    let body_statements = parse_statements_range(tokens, current_index + 1, block_end)?;
    current_index = block_end + 1;
    Ok((
        Statement::ForLoop {
            initialization: None,
            condition: None,
            update: None,
            body: Box::new(BlockStatement { statements: body_statements }),
        },
        current_index - start_index,
    ))
}

pub fn parse_when_statement(tokens: &[Token], start_index: usize, _scope: &ScopeManager) -> ParseResult<Statement> {
    let mut current_index = start_index;

    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordWhen {
        return Err("Expected 'when' keyword to start statement.".into());
    }

    current_index += 1;

    let expr_start = current_index;
    while current_index < tokens.len() && tokens[current_index].lexeme != "{" {
        current_index += 1;
    }

    if current_index >= tokens.len() || tokens[current_index].lexeme != "{" {
        return Err("When statement expected '{' to begin body.".into());
    }

    let expr_end = current_index;
    let open_brace = current_index;
    let block_end = find_block_end(tokens, open_brace)?;

    let when_expression = parse_expression_range(tokens, expr_start, expr_end)?;
    let mut cases = Vec::new();
    let mut else_block = None;

    current_index = open_brace + 1;
    while current_index < block_end {
        if tokens[current_index].kind() == TokenKind::NewLine {
            current_index += 1;
            continue;
        }
        if tokens[current_index].kind() == TokenKind::KeywordIs {
            current_index += 1;
            let case_expr_start = current_index;
            while current_index < block_end && tokens[current_index].lexeme != "{" {
                current_index += 1;
            }
            if current_index >= block_end {
                return Err("when case expected '{' after 'is ...'.".into());
            }
            let case_exprs = parse_expression_list(tokens, case_expr_start, current_index)?;
            let case_block_end = find_block_end(tokens, current_index)?;
            let case_statements = parse_statements_range(tokens, current_index + 1, case_block_end)?;
            cases.push((
                case_exprs,
                Box::new(BlockStatement {
                    statements: case_statements,
                }),
            ));
            current_index = case_block_end + 1;
            continue;
        }
        if tokens[current_index].kind() == TokenKind::KeywordElse {
            current_index += 1;
            if current_index >= block_end || tokens[current_index].lexeme != "{" {
                return Err("when else expected '{'.".into());
            }
            let else_end = find_block_end(tokens, current_index)?;
            let else_statements = parse_statements_range(tokens, current_index + 1, else_end)?;
            else_block = Some(Box::new(BlockStatement {
                statements: else_statements,
            }));
            current_index = else_end + 1;
            continue;
        }
        return Err(format!(
            "Unexpected token in when block: {:?} ('{}')",
            tokens[current_index].kind(),
            tokens[current_index].lexeme
        ));
    }

    current_index = block_end + 1;
    let stmt = Statement::WhenBlock {
        when_expression: Box::new(when_expression),
        cases,
        else_block,
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
    let then_statements = parse_statements_range(tokens, cursor + 1, then_end)?;
    let mut consumed_to = then_end + 1;
    let mut else_block = None;

    if consumed_to < tokens.len() && tokens[consumed_to].kind() == TokenKind::KeywordElse {
        consumed_to += 1;
        if consumed_to < tokens.len() && tokens[consumed_to].kind() == TokenKind::KeywordIf {
            let (else_if_stmt, else_if_consumed) = parse_if_statement(tokens, consumed_to)?;
            consumed_to += else_if_consumed;
            else_block = Some(Box::new(BlockStatement {
                statements: vec![else_if_stmt],
            }));
        } else {
            if consumed_to >= tokens.len() || tokens[consumed_to].lexeme != "{" {
                return Err("Else branch expected '{'.".into());
            }
            let else_end = find_block_end(tokens, consumed_to)?;
            let else_statements = parse_statements_range(tokens, consumed_to + 1, else_end)?;
            consumed_to = else_end + 1;
            else_block = Some(Box::new(BlockStatement {
                statements: else_statements,
            }));
        }
    }

    let condition = parse_expression_range(tokens, expr_start, cursor)?;
    Ok((
        Statement::IfStatement {
            condition: Box::new(condition),
            then_block: Box::new(BlockStatement {
                statements: then_statements,
            }),
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
    let body_statements = parse_statements_range(tokens, cursor + 1, block_end)?;
    Ok((
        Statement::WhileLoop {
            condition: Box::new(condition),
            body: Box::new(BlockStatement {
                statements: body_statements,
            }),
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
    let body_statements = parse_statements_range(tokens, open + 1, block_end)?;
    Ok((
        Statement::LoopStatement {
            body: Box::new(BlockStatement {
                statements: body_statements,
            }),
        },
        block_end + 1 - start_index,
    ))
}

pub fn parse_return_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordReturn {
        return Err("Expected 'return' keyword.".into());
    }

    let expr_start = start_index + 1;
    if expr_start + 1 < tokens.len()
        && tokens[expr_start].kind() == TokenKind::Identifier
        && tokens[expr_start].lexeme == "error"
        && tokens[expr_start + 1].kind() == TokenKind::Identifier
    {
        return Ok((
            Statement::ReturnError {
                code: tokens[expr_start + 1].lexeme.clone(),
            },
            3,
        ));
    }

    let mut cursor = expr_start;
    while cursor < tokens.len() {
        if tokens[cursor].kind() == TokenKind::NewLine || tokens[cursor].lexeme == "}" {
            break;
        }
        cursor += 1;
    }

    let value = if cursor > expr_start {
        Some(Box::new(parse_expression_range(tokens, expr_start, cursor)?))
    } else {
        None
    };

    Ok((
        Statement::ReturnStatement { value },
        (cursor - start_index).max(1),
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

fn parse_call_expression(tokens: &[Token], start: usize, end: usize) -> Result<(String, Vec<crate::ast_nodes::Expression>), String> {
    if start + 2 >= end {
        return Err("Danger call expected 'name(...)'.".into());
    }
    if tokens[start].kind() != TokenKind::Identifier {
        return Err("Danger call must start with function name.".into());
    }
    if tokens[start + 1].lexeme != "(" {
        return Err("Danger call expected '(' after function name.".into());
    }
    if tokens[end - 1].lexeme != ")" {
        return Err("Danger call expected ')'.".into());
    }

    let call_name = tokens[start].lexeme.clone();
    let mut args = Vec::new();
    let mut arg_start = start + 2;
    let mut depth = 0usize;
    let mut i = start + 2;
    while i < end - 1 {
        let t = &tokens[i];
        if t.lexeme == "(" {
            depth += 1;
        } else if t.lexeme == ")" {
            depth = depth.saturating_sub(1);
        } else if t.lexeme == "," && depth == 0 {
            if arg_start < i {
                args.push(parse_expression_range(tokens, arg_start, i)?);
            }
            arg_start = i + 1;
        }
        i += 1;
    }
    if arg_start < end - 1 {
        args.push(parse_expression_range(tokens, arg_start, end - 1)?);
    }
    Ok((call_name, args))
}

pub fn parse_identifier_led_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    let mut line_end = start_index;
    while line_end < tokens.len() {
        if tokens[line_end].kind() == TokenKind::NewLine || tokens[line_end].lexeme == "}" {
            break;
        }
        line_end += 1;
    }

    let on_idx = (start_index..line_end).find(|&i| {
        tokens[i].kind() == TokenKind::KeywordOnError
            && i + 1 < line_end
            && tokens[i + 1].lexeme == "error"
    });

    if let Some(on_idx) = on_idx {
        let block_open = on_idx + 2;
        if block_open >= tokens.len() || tokens[block_open].lexeme != "{" {
            return Err("on error expected '{'.".into());
        }
        let block_end = find_block_end(tokens, block_open)?;
        let on_error_statements = parse_statements_range(tokens, block_open + 1, block_end)?;

        if start_index + 2 < on_idx
            && tokens[start_index].kind() == TokenKind::Identifier
            && tokens[start_index + 1].kind() == TokenKind::OpAssignment
        {
            let target = tokens[start_index].lexeme.clone();
            let (call_name, args) = parse_call_expression(tokens, start_index + 2, on_idx)?;
            return Ok((
                Statement::DangerAssignOnError {
                    target,
                    call_name,
                    args,
                    on_error: Box::new(BlockStatement {
                        statements: on_error_statements,
                    }),
                },
                block_end + 1 - start_index,
            ));
        }

        let (call_name, args) = parse_call_expression(tokens, start_index, on_idx)?;
        return Ok((
            Statement::DangerCallOnError {
                call_name,
                args,
                on_error: Box::new(BlockStatement {
                    statements: on_error_statements,
                }),
            },
            block_end + 1 - start_index,
        ));
    }

    parse_assignment_statement(tokens, start_index)
}

pub fn parse_new_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordNew {
        return Err("Expected 'new' keyword.".into());
    }
    if start_index + 2 >= tokens.len() {
        return Err("Incomplete variable declaration after 'new'.".into());
    }
    let mut idx = start_index + 1;
    let mut declared_type: Option<String> = None;

    // Supports both:
    // new x = 1
    // new Int x = 1
    if idx + 2 < tokens.len()
        && tokens[idx].kind() == TokenKind::Identifier
        && tokens[idx + 1].kind() == TokenKind::Identifier
        && tokens[idx + 2].kind() == TokenKind::OpAssignment
    {
        declared_type = Some(tokens[idx].lexeme.clone());
        idx += 1;
    }

    if idx >= tokens.len() || tokens[idx].kind() != TokenKind::Identifier {
        return Err("Variable declaration expected identifier after 'new'.".into());
    }
    if idx + 1 >= tokens.len() || tokens[idx + 1].kind() != TokenKind::OpAssignment {
        return Err("Variable declaration expected '=' after identifier.".into());
    }

    let name = tokens[idx].lexeme.clone();
    let expr_start = idx + 2;
    let mut cursor = expr_start;
    while cursor < tokens.len() {
        if tokens[cursor].kind() == TokenKind::NewLine || tokens[cursor].lexeme == "}" {
            break;
        }
        cursor += 1;
    }
    let value = parse_expression_range(tokens, expr_start, cursor)?;
    Ok((
        Statement::VarDecl {
            name,
            value: Box::new(value),
            is_fixed: false,
            declared_type,
        },
        (cursor - start_index).max(4),
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
