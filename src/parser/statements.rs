// ================================================
// Parser Logic Helpers (Rust)
// File: src/parser/statements.rs
// ----------------------------------------------------------------
use crate::ast_nodes::{
    BlockStatement, BorrowMode, Expression, ForLoopStyle, FunctionParam, LabelVariant,
    LegacyForParts, Location, MemoryKind, ScopeManager, Statement, StructField, StructMethod,
};
use crate::common_types::{Token, TokenKind};

use super::expressions::parse_expression_range;
use super::parse_statements_range;

/// Core type for a parser function: consumes tokens and returns the resulting AST node and the count of consumed tokens.
pub type ParseResult<T> = Result<(T, usize), String>;

fn parse_err(code: &str, message: impl AsRef<str>) -> String {
    format!("[{}] {}", code, message.as_ref())
}

fn parse_qualified_identifier(tokens: &[Token], start: usize) -> Option<(String, usize)> {
    if start >= tokens.len() || tokens[start].kind() != TokenKind::Identifier {
        return None;
    }
    if start + 2 < tokens.len()
        && tokens[start + 1].lexeme == "."
        && tokens[start + 2].kind() == TokenKind::Identifier
    {
        return Some((
            format!("{}.{}", tokens[start].lexeme, tokens[start + 2].lexeme),
            3,
        ));
    }
    Some((tokens[start].lexeme.clone(), 1))
}

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

fn render_token_slice(tokens: &[Token], start: usize, end: usize) -> String {
    let mut out = String::new();
    let mut prev: Option<&str> = None;

    for token in &tokens[start..end] {
        let lexeme = token.lexeme.as_str();
        if token.kind() == TokenKind::Whitespace || token.kind() == TokenKind::NewLine {
            continue;
        }

        let need_space = if out.is_empty() {
            false
        } else {
            !matches!(
                (prev.unwrap_or(""), lexeme),
                ("(", _)
                    | ("[", _)
                    | ("{", _)
                    | (_, ")")
                    | (_, "]")
                    | (_, "}")
                    | (_, ",")
                    | (_, ";")
                    | (_, "(")
                    | (_, "[")
                    | (_, "++")
                    | (_, "--")
                    | (".", _)
                    | (_, ".")
            )
        };

        if need_space {
            out.push(' ');
        }
        out.push_str(lexeme);
        prev = Some(lexeme);
    }

    out
}

fn parse_parenthesized_type_name(tokens: &[Token], start: usize) -> Option<(String, usize)> {
    if start >= tokens.len() || tokens[start].kind() != TokenKind::Identifier {
        return None;
    }
    let base = tokens[start].lexeme.as_str();
    if base != "Task"
        && base != "Channel"
        && base != "Buffer"
        && base != "Interrupt"
        && base != "Canvas"
        && base != "Window"
    {
        return None;
    }
    if start + 1 >= tokens.len() || tokens[start + 1].lexeme != "(" {
        return if matches!(base, "Task" | "Interrupt" | "Canvas" | "Window") {
            Some((base.to_string(), start + 1))
        } else {
            None
        };
    }
    let mut cursor = start + 2;
    let mut depth = 1usize;
    while cursor < tokens.len() && depth > 0 {
        if tokens[cursor].lexeme == "(" {
            depth += 1;
        } else if tokens[cursor].lexeme == ")" {
            depth -= 1;
        }
        cursor += 1;
    }
    if depth != 0 || cursor <= start + 3 {
        return None;
    }
    let inner = render_token_slice(tokens, start + 2, cursor - 1);
    if inner.trim().is_empty() {
        return None;
    }
    Some((format!("{base}({})", inner.trim()), cursor))
}

fn parse_type_name_at(tokens: &[Token], start: usize) -> Option<(String, usize)> {
    if let Some(parsed) = parse_parenthesized_type_name(tokens, start) {
        return Some(parsed);
    }
    if let Some((qualified, consumed)) = parse_qualified_identifier(tokens, start) {
        let next = start + consumed;
        if next < tokens.len()
            && tokens[next].kind() == TokenKind::Identifier
            && tokens[next].lexeme == "List"
        {
            return Some((format!("{qualified} List"), next + 1));
        }
        return Some((qualified, next));
    }
    None
}

fn find_block_end(tokens: &[Token], open_brace_index: usize) -> Result<usize, String> {
    if open_brace_index >= tokens.len() || tokens[open_brace_index].lexeme != "{" {
        return Err(parse_err("SC-PARSE-101", "expected '{'."));
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
        return Err(parse_err(
            "SC-PARSE-102",
            "unterminated block: missing '}'.",
        ));
    }
    Ok(current - 1)
}

fn parse_braced_block(
    tokens: &[Token],
    open_brace_index: usize,
) -> Result<(Box<BlockStatement>, usize), String> {
    let block_end = find_block_end(tokens, open_brace_index)?;
    let statements = parse_statements_range(tokens, open_brace_index + 1, block_end)?;
    Ok((Box::new(BlockStatement { statements }), block_end))
}

pub fn parse_control_keyword_statement(
    tokens: &[Token],
    start_index: usize,
) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    let stmt = match tokens[start_index].kind() {
        TokenKind::KeywordBreak => Statement::BreakStatement { loc },
        TokenKind::KeywordContinue => Statement::ContinueStatement { loc },
        TokenKind::KeywordPass => Statement::PassStatement { loc },
        _ => {
            return Err(parse_err(
                "SC-PARSE-158",
                "expected break/continue/pass keyword.",
            ));
        }
    };

    let mut cursor = start_index + 1;
    while cursor < tokens.len()
        && tokens[cursor].kind() != TokenKind::NewLine
        && tokens[cursor].lexeme != "}"
    {
        cursor += 1;
    }
    Ok((stmt, (cursor - start_index).max(1)))
}

pub fn parse_function_declaration(
    tokens: &[Token],
    start_index: usize,
    _scope: &ScopeManager,
) -> ParseResult<Statement> {
    parse_function_declaration_inner(tokens, start_index, false, false)
}

pub fn parse_external_function_declaration(
    tokens: &[Token],
    start_index: usize,
) -> ParseResult<Statement> {
    if tokens.get(start_index).map(Token::kind) != Some(TokenKind::KeywordExternal) {
        return Err(parse_err(
            "SC-PARSE-227",
            "expected 'external' before external function declaration.",
        ));
    }
    parse_function_declaration_inner(tokens, start_index + 1, false, true)
        .map(|(statement, consumed)| (statement, consumed + 1))
}

pub fn parse_external_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    if tokens.get(start_index).map(Token::kind) != Some(TokenKind::KeywordExternal) {
        return Err(parse_err(
            "SC-PARSE-227",
            "expected 'external' before external declaration.",
        ));
    }
    match tokens.get(start_index + 1).map(Token::kind) {
        Some(TokenKind::Identifier)
            if tokens
                .get(start_index + 1)
                .is_some_and(|token| token.lexeme == "resource") =>
        {
            let Some(name) = tokens.get(start_index + 2) else {
                return Err(parse_err(
                    "SC-PARSE-232",
                    "external resource declaration requires a type name.",
                ));
            };
            if name.kind() != TokenKind::Identifier {
                return Err(parse_err(
                    "SC-PARSE-232",
                    "external resource declaration requires an identifier type name.",
                ));
            }
            if tokens
                .get(start_index + 3)
                .is_some_and(|token| token.kind() != TokenKind::NewLine && token.lexeme != "}")
            {
                return Err(parse_err(
                    "SC-PARSE-233",
                    "external resource declaration must end after its type name.",
                ));
            }
            Ok((
                Statement::StructDecl {
                    name: name.lexeme.clone(),
                    fields: Vec::new(),
                    methods: Vec::new(),
                    is_local: false,
                    is_external: true,
                    is_resource: true,
                    loc: Location {
                        line: tokens[start_index].line,
                        column: tokens[start_index].col,
                    },
                },
                3,
            ))
        }
        Some(TokenKind::KeywordStruct) => {
            parse_struct_declaration_inner(tokens, start_index + 1, false, true)
                .map(|(statement, consumed)| (statement, consumed + 1))
        }
        Some(TokenKind::KeywordFn) | Some(TokenKind::Identifier)
            if tokens.get(start_index + 1).is_some_and(|token| {
                token.kind() == TokenKind::KeywordFn || token.lexeme == "danger"
            }) =>
        {
            parse_external_function_declaration(tokens, start_index)
        }
        _ => Err(parse_err(
            "SC-PARSE-230",
            "external must prefix 'fn', 'danger fn', 'struct', or 'resource'.",
        )),
    }
}

fn parse_function_declaration_inner(
    tokens: &[Token],
    start_index: usize,
    is_local: bool,
    is_external: bool,
) -> ParseResult<Statement> {
    let mut current_index = start_index;
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    let mut is_danger = false;

    if current_index < tokens.len()
        && tokens[current_index].kind() == TokenKind::Identifier
        && tokens[current_index].lexeme == "danger"
    {
        is_danger = true;
        current_index += 1;
    }

    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordFn {
        return Err(parse_err("SC-PARSE-103", "expected 'fn' keyword."));
    }

    let function_name = if current_index + 1 < tokens.len()
        && tokens[current_index + 1].kind() == TokenKind::Identifier
    {
        tokens[current_index + 1].lexeme.clone()
    } else {
        return Err(parse_err(
            "SC-PARSE-104",
            "function definition must be followed by an identifier (function name).",
        ));
    };

    if current_index + 2 >= tokens.len()
        || tokens[current_index + 2].kind() != TokenKind::OpPunctuation
        || tokens[current_index + 2].lexeme != "("
    {
        return Err(parse_err(
            "SC-PARSE-105",
            "function signature expected '('.",
        ));
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

            let mut borrow = BorrowMode::Value;
            let mut type_index = current_index;
            if tokens[type_index].kind() == TokenKind::KeywordEdit {
                borrow = BorrowMode::EditMutable;
                type_index += 1;
            } else if tokens[type_index].kind() == TokenKind::KeywordView {
                borrow = BorrowMode::View;
                type_index += 1;
            } else if tokens[type_index].kind() == TokenKind::KeywordMove {
                borrow = BorrowMode::Move;
                type_index += 1;
            }

            if let Some((param_type, name_index)) = parse_type_name_at(tokens, type_index)
                && name_index < tokens.len()
                && tokens[name_index].kind() == TokenKind::Identifier
                && (name_index + 1 >= tokens.len()
                    || tokens[name_index + 1].lexeme == ","
                    || tokens[name_index + 1].lexeme == ")")
                && tokens[type_index].lexeme != tokens[name_index].lexeme
            {
                params.push(FunctionParam {
                    param_type: Some(param_type),
                    name: tokens[name_index].lexeme.clone(),
                    borrow,
                });
                current_index = name_index + 1;
                continue;
            }

            if tokens[current_index].kind() == TokenKind::Identifier {
                params.push(FunctionParam {
                    param_type: None,
                    name: tokens[current_index].lexeme.clone(),
                    borrow: BorrowMode::Value,
                });
            }
            current_index += 1;
        }
        if current_index < tokens.len() && tokens[current_index].lexeme == ")" {
            current_index += 1;
        } else {
            return Err(parse_err(
                "SC-PARSE-106",
                "function signature expected ')' after parameters.",
            ));
        }
    }

    let mut returns = None;
    let mut uses_returns_keyword = false;
    if current_index < tokens.len() && tokens[current_index].lexeme == "returns" {
        uses_returns_keyword = true;
        current_index += 1;
        if let Some((return_type, next_index)) = parse_type_name_at(tokens, current_index) {
            returns = Some(return_type);
            current_index = next_index;
        } else {
            return Err(parse_err(
                "SC-PARSE-163",
                "expected return type after 'returns'.",
            ));
        }
    } else if !is_external
        && let Some((return_type, next_index)) = parse_type_name_at(tokens, current_index)
        && next_index < tokens.len()
        && tokens[next_index].lexeme == "{"
    {
        returns = Some(return_type);
        current_index = next_index;
    }

    if is_external {
        if params
            .iter()
            .any(|parameter| parameter.param_type.is_none())
        {
            return Err(parse_err(
                "SC-PARSE-228",
                "external function parameters require explicit types.",
            ));
        }
        if current_index < tokens.len()
            && tokens[current_index].kind() != TokenKind::NewLine
            && tokens[current_index].lexeme != "}"
        {
            return Err(parse_err(
                "SC-PARSE-229",
                "external function declaration must end after its return type.",
            ));
        }
        let statement = Statement::FunctionDef {
            name: function_name,
            params,
            body: Vec::new().into(),
            returns,
            uses_returns_keyword,
            is_danger,
            is_local: false,
            is_external: true,
            loc,
        };
        return Ok((statement, current_index - start_index));
    }

    if current_index >= tokens.len()
        || tokens[current_index].kind() != TokenKind::OpPunctuation
        || tokens[current_index].lexeme != "{"
    {
        return Err(parse_err(
            "SC-PARSE-107",
            "function signature expected '{' to begin the body block.",
        ));
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
        uses_returns_keyword,
        is_danger,
        is_local,
        is_external: false,
        loc,
    };

    Ok((stmt, current_index - start_index))
}

pub fn parse_local_prefixed_declaration(
    tokens: &[Token],
    start_index: usize,
    _scope: &ScopeManager,
) -> ParseResult<Statement> {
    if start_index + 1 >= tokens.len() {
        return Err(parse_err(
            "SC-PARSE-161",
            "local must prefix fn/struct/label/tag.",
        ));
    }
    match tokens[start_index + 1].kind() {
        TokenKind::KeywordFn => {
            parse_function_declaration_inner(tokens, start_index + 1, true, false)
                .map(|(stmt, consumed)| (stmt, consumed + 1))
        }
        TokenKind::KeywordStruct => {
            parse_struct_declaration_inner(tokens, start_index + 1, true, false)
                .map(|(stmt, consumed)| (stmt, consumed + 1))
        }
        TokenKind::KeywordLabel => parse_label_declaration_inner(tokens, start_index + 1, true)
            .map(|(stmt, consumed)| (stmt, consumed + 1)),
        TokenKind::KeywordTag => parse_tag_declaration_inner(tokens, start_index + 1, true)
            .map(|(stmt, consumed)| (stmt, consumed + 1)),
        _ => Err(parse_err(
            "SC-PARSE-162",
            "local may prefix only fn/struct/label/tag declarations.",
        )),
    }
}

pub fn parse_for_loop(
    tokens: &[Token],
    start_index: usize,
    _scope: &ScopeManager,
) -> ParseResult<Statement> {
    let mut current_index = start_index;
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };

    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordFor {
        return Err(parse_err(
            "SC-PARSE-108",
            "expected 'for' keyword to start loop.",
        ));
    }

    current_index += 1;

    if current_index < tokens.len() && tokens[current_index].kind() == TokenKind::Identifier {
        let loop_var = tokens[current_index].lexeme.clone();
        current_index += 1;
        if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordIn {
            return Err(parse_err(
                "SC-PARSE-109",
                "for loop expected 'in' after iterator variable.",
            ));
        }
        current_index += 1;
        let expr_start = current_index;
        while current_index < tokens.len() && tokens[current_index].lexeme != "{" {
            current_index += 1;
        }
        if current_index >= tokens.len() {
            return Err(parse_err(
                "SC-PARSE-110",
                "for-in loop expected '{' to begin body.",
            ));
        }
        let collection_expr = parse_expression_range(tokens, expr_start, current_index)?;
        let block_end = find_block_end(tokens, current_index)?;
        let body_statements = parse_statements_range(tokens, current_index + 1, block_end)?;
        current_index = block_end + 1;
        let stmt = Statement::ForLoop {
            initialization: Some(Box::new(crate::ast_nodes::Expression::VariableReference(
                loop_var,
            ))),
            condition: Some(Box::new(collection_expr)),
            update: None,
            legacy_parts: None,
            style: ForLoopStyle::ForIn,
            body: Box::new(BlockStatement {
                statements: body_statements,
            }),
            loc,
        };
        return Ok((stmt, current_index - start_index));
    }

    if current_index >= tokens.len()
        || tokens[current_index].kind() != TokenKind::OpPunctuation
        || tokens[current_index].lexeme != "("
    {
        return Err(parse_err(
            "SC-PARSE-111",
            "for loop expected iterator variable or '(' after keyword.",
        ));
    }
    current_index += 1;
    let init_start = current_index;
    while current_index < tokens.len() && tokens[current_index].lexeme != ";" {
        current_index += 1;
    }
    if current_index >= tokens.len() || tokens[current_index].lexeme != ";" {
        return Err(parse_err(
            "SC-PARSE-112",
            "for loop expected ';' after initialization.",
        ));
    }
    let init_end = current_index;
    current_index += 1;
    let cond_start = current_index;
    while current_index < tokens.len() && tokens[current_index].lexeme != ";" {
        current_index += 1;
    }
    if current_index >= tokens.len() || tokens[current_index].lexeme != ";" {
        return Err(parse_err(
            "SC-PARSE-113",
            "for loop expected ';' after condition.",
        ));
    }
    let cond_end = current_index;
    current_index += 1;
    let update_start = current_index;
    while current_index < tokens.len() && tokens[current_index].lexeme != ")" {
        current_index += 1;
    }
    if current_index >= tokens.len() || tokens[current_index].lexeme != ")" {
        return Err(parse_err("SC-PARSE-114", "for loop expected ')'."));
    }
    let update_end = current_index;
    current_index += 1;
    if current_index >= tokens.len() || tokens[current_index].lexeme != "{" {
        return Err(parse_err(
            "SC-PARSE-115",
            "for loop expected '{' to begin body.",
        ));
    }
    let block_end = find_block_end(tokens, current_index)?;
    let body_statements = parse_statements_range(tokens, current_index + 1, block_end)?;
    current_index = block_end + 1;
    Ok((
        Statement::ForLoop {
            initialization: None,
            condition: None,
            update: None,
            legacy_parts: Some(LegacyForParts {
                initialization: render_token_slice(tokens, init_start, init_end),
                condition: render_token_slice(tokens, cond_start, cond_end),
                update: render_token_slice(tokens, update_start, update_end),
            }),
            style: ForLoopStyle::LegacyCStyle,
            body: Box::new(BlockStatement {
                statements: body_statements,
            }),
            loc,
        },
        current_index - start_index,
    ))
}

pub fn parse_iterate_loop(
    tokens: &[Token],
    start_index: usize,
    _scope: &ScopeManager,
) -> ParseResult<Statement> {
    let mut current_index = start_index;
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };

    if current_index >= tokens.len()
        || tokens[current_index].kind() != TokenKind::Identifier
        || tokens[current_index].lexeme != "iterate"
    {
        return Err(parse_err("SC-PARSE-149", "expected 'iterate' keyword."));
    }
    current_index += 1;

    let collection_start = current_index;
    while current_index < tokens.len()
        && !(tokens[current_index].kind() == TokenKind::Identifier
            && tokens[current_index].lexeme == "as")
    {
        current_index += 1;
    }
    if current_index >= tokens.len() || current_index == collection_start {
        return Err(parse_err(
            "SC-PARSE-150",
            "iterate loop expected collection before 'as'.",
        ));
    }
    let collection_expr = parse_expression_range(tokens, collection_start, current_index)?;
    current_index += 1; // skip 'as'

    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::Identifier {
        return Err(parse_err(
            "SC-PARSE-151",
            "iterate loop expected item identifier after 'as'.",
        ));
    }
    let loop_var = tokens[current_index].lexeme.clone();
    current_index += 1;

    if current_index >= tokens.len() || tokens[current_index].lexeme != "{" {
        return Err(parse_err(
            "SC-PARSE-152",
            "iterate loop expected '{' to begin body.",
        ));
    }
    let block_end = find_block_end(tokens, current_index)?;
    let body_statements = parse_statements_range(tokens, current_index + 1, block_end)?;
    current_index = block_end + 1;

    Ok((
        Statement::ForLoop {
            initialization: Some(Box::new(crate::ast_nodes::Expression::VariableReference(
                loop_var,
            ))),
            condition: Some(Box::new(collection_expr)),
            update: None,
            legacy_parts: None,
            style: ForLoopStyle::IterateAs,
            body: Box::new(BlockStatement {
                statements: body_statements,
            }),
            loc,
        },
        current_index - start_index,
    ))
}

pub fn parse_when_statement(
    tokens: &[Token],
    start_index: usize,
    _scope: &ScopeManager,
) -> ParseResult<Statement> {
    let mut current_index = start_index;
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };

    if current_index >= tokens.len() || tokens[current_index].kind() != TokenKind::KeywordWhen {
        return Err(parse_err(
            "SC-PARSE-116",
            "expected 'when' keyword to start statement.",
        ));
    }

    current_index += 1;

    let expr_start = current_index;
    while current_index < tokens.len() && tokens[current_index].lexeme != "{" {
        current_index += 1;
    }

    if current_index >= tokens.len() || tokens[current_index].lexeme != "{" {
        return Err(parse_err(
            "SC-PARSE-117",
            "when statement expected '{' to begin body.",
        ));
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
                return Err(parse_err(
                    "SC-PARSE-118",
                    "when case expected '{' after 'is ...'.",
                ));
            }
            let case_exprs = parse_expression_list(tokens, case_expr_start, current_index)?;
            let case_block_end = find_block_end(tokens, current_index)?;
            let case_statements =
                parse_statements_range(tokens, current_index + 1, case_block_end)?;
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
                return Err(parse_err("SC-PARSE-119", "when else expected '{'."));
            }
            let else_end = find_block_end(tokens, current_index)?;
            let else_statements = parse_statements_range(tokens, current_index + 1, else_end)?;
            else_block = Some(Box::new(BlockStatement {
                statements: else_statements,
            }));
            current_index = else_end + 1;
            continue;
        }
        return Err(parse_err(
            "SC-PARSE-120",
            format!(
                "unexpected token in when block: {:?} ('{}')",
                tokens[current_index].kind(),
                tokens[current_index].lexeme
            ),
        ));
    }

    current_index = block_end + 1;
    let stmt = Statement::WhenBlock {
        when_expression: Box::new(when_expression),
        cases,
        else_block,
        loc,
    };

    Ok((stmt, current_index - start_index))
}

pub fn parse_if_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordIf {
        return Err(parse_err("SC-PARSE-121", "expected 'if' keyword."));
    }
    let expr_start = start_index + 1;
    let mut cursor = expr_start;
    while cursor < tokens.len() && tokens[cursor].lexeme != "{" {
        cursor += 1;
    }
    if cursor >= tokens.len() {
        return Err(parse_err("SC-PARSE-122", "if statement expected '{'."));
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
                return Err(parse_err("SC-PARSE-123", "else branch expected '{'."));
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
            loc,
        },
        consumed_to - start_index,
    ))
}

pub fn parse_while_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordWhile {
        return Err(parse_err("SC-PARSE-124", "expected 'while' keyword."));
    }
    let expr_start = start_index + 1;
    let mut cursor = expr_start;
    while cursor < tokens.len() && tokens[cursor].lexeme != "{" {
        cursor += 1;
    }
    if cursor >= tokens.len() {
        return Err(parse_err("SC-PARSE-125", "while statement expected '{'."));
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
            loc,
        },
        block_end + 1 - start_index,
    ))
}

pub fn parse_loop_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordLoop {
        return Err(parse_err("SC-PARSE-126", "expected 'loop' keyword."));
    }
    let open = start_index + 1;
    if open >= tokens.len() || tokens[open].lexeme != "{" {
        return Err(parse_err(
            "SC-PARSE-127",
            "loop statement expected '{' after 'loop'.",
        ));
    }
    let block_end = find_block_end(tokens, open)?;
    let body_statements = parse_statements_range(tokens, open + 1, block_end)?;
    Ok((
        Statement::LoopStatement {
            body: Box::new(BlockStatement {
                statements: body_statements,
            }),
            loc,
        },
        block_end + 1 - start_index,
    ))
}

pub fn parse_return_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordReturn {
        return Err(parse_err("SC-PARSE-128", "expected 'return' keyword."));
    }

    let expr_start = start_index + 1;
    if expr_start + 1 < tokens.len()
        && tokens[expr_start].kind() == TokenKind::Identifier
        && tokens[expr_start].lexeme == "error"
        && tokens[expr_start + 1].kind() == TokenKind::Identifier
    {
        if expr_start + 3 < tokens.len()
            && tokens[expr_start + 2].lexeme == "."
            && tokens[expr_start + 3].kind() == TokenKind::Identifier
        {
            return Ok((
                Statement::ReturnError {
                    code: format!(
                        "{}.{}",
                        tokens[expr_start + 1].lexeme,
                        tokens[expr_start + 3].lexeme
                    ),
                    loc,
                },
                5,
            ));
        }
        return Ok((
            Statement::ReturnError {
                code: tokens[expr_start + 1].lexeme.clone(),
                loc,
            },
            3,
        ));
    }

    let mut cursor = expr_start;
    let mut depth_curly = 0usize;
    let mut depth_round = 0usize;
    let mut depth_square = 0usize;
    while cursor < tokens.len() {
        let lx = tokens[cursor].lexeme.as_str();
        if lx == "{" {
            depth_curly += 1;
        } else if lx == "}" {
            if depth_curly > 0 {
                depth_curly -= 1;
            } else if depth_round == 0 && depth_square == 0 {
                break;
            }
        } else if lx == "(" {
            depth_round += 1;
        } else if lx == ")" {
            depth_round = depth_round.saturating_sub(1);
        } else if lx == "[" {
            depth_square += 1;
        } else if lx == "]" {
            depth_square = depth_square.saturating_sub(1);
        }
        if tokens[cursor].kind() == TokenKind::NewLine
            && depth_curly == 0
            && depth_round == 0
            && depth_square == 0
        {
            break;
        }
        cursor += 1;
    }

    let value = if cursor > expr_start {
        Some(Box::new(parse_expression_range(
            tokens, expr_start, cursor,
        )?))
    } else {
        None
    };

    Ok((
        Statement::ReturnStatement { value, loc },
        (cursor - start_index).max(1),
    ))
}

pub fn parse_assignment_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index + 1 >= tokens.len() {
        return Err(parse_err("SC-PARSE-129", "incomplete assignment."));
    }
    if tokens[start_index].kind() != TokenKind::Identifier
        && tokens[start_index].kind() != TokenKind::KeywordMy
    {
        return Err(parse_err(
            "SC-PARSE-130",
            "assignment must start with identifier.",
        ));
    }
    if tokens[start_index + 1].kind() != TokenKind::OpAssignment {
        return Err(parse_err("SC-PARSE-131", "expected assignment operator."));
    }
    let target_name = tokens[start_index].lexeme.clone();
    let assignment_op = tokens[start_index + 1].lexeme.clone();
    let mut cursor = start_index + 2;
    while cursor < tokens.len() {
        if tokens[cursor].kind() == TokenKind::NewLine || tokens[cursor].lexeme == "}" {
            break;
        }
        cursor += 1;
    }
    let mut value = parse_expression_range(tokens, start_index + 2, cursor)?;
    if assignment_op != "=" {
        let op = assignment_op
            .strip_suffix('=')
            .ok_or_else(|| parse_err("SC-PARSE-131", "invalid assignment operator."))?;
        value = Expression::BinaryOp {
            op: op.to_string(),
            left: Box::new(Expression::VariableReference(target_name.clone())),
            right: Some(Box::new(value)),
        };
    }
    Ok((
        Statement::Assignment {
            target: target_name,
            value: Box::new(value),
            loc,
        },
        (cursor - start_index).max(2),
    ))
}

pub fn parse_task_or_channel_declaration(
    tokens: &[Token],
    start_index: usize,
) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    let Some((declared_type, name_index)) = parse_parenthesized_type_name(tokens, start_index)
    else {
        return Err(parse_err(
            "SC-PARSE-175",
            "expected Task, Channel, or Interrupt declaration.",
        ));
    };
    if name_index >= tokens.len() || tokens[name_index].kind() != TokenKind::Identifier {
        return Err(parse_err(
            "SC-PARSE-176",
            "capability declaration expected identifier name.",
        ));
    }
    if name_index + 1 >= tokens.len()
        || tokens[name_index + 1].kind() != TokenKind::OpAssignment
        || tokens[name_index + 1].lexeme != "="
    {
        return Err(parse_err(
            "SC-PARSE-177",
            "capability declaration expected '=' after name.",
        ));
    }
    let mut cursor = name_index + 2;
    while cursor < tokens.len()
        && tokens[cursor].kind() != TokenKind::NewLine
        && tokens[cursor].lexeme != "}"
    {
        cursor += 1;
    }
    let value = parse_expression_range(tokens, name_index + 2, cursor)?;
    Ok((
        Statement::VarDecl {
            name: tokens[name_index].lexeme.clone(),
            value: Box::new(value),
            is_constant: false,
            declared_type: Some(declared_type),
            on_error: None,
            loc,
        },
        cursor - start_index,
    ))
}

fn parse_call_expression(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Result<(String, Vec<crate::ast_nodes::Expression>), String> {
    if start + 2 >= end {
        return Err(parse_err(
            "SC-PARSE-132",
            "danger call expected 'name(...)'.",
        ));
    }
    if tokens[start].kind() != TokenKind::Identifier {
        return Err(parse_err(
            "SC-PARSE-133",
            "danger call must start with function name.",
        ));
    }
    let (call_name, open_index) = if start + 3 < end
        && tokens[start + 1].lexeme == "."
        && tokens[start + 2].kind() == TokenKind::Identifier
        && tokens[start + 3].lexeme == "("
    {
        (
            format!("{}.{}", tokens[start].lexeme, tokens[start + 2].lexeme),
            start + 3,
        )
    } else {
        (tokens[start].lexeme.clone(), start + 1)
    };
    if tokens[open_index].lexeme != "(" {
        return Err(parse_err(
            "SC-PARSE-134",
            "danger call expected '(' after function name.",
        ));
    }
    if tokens[end - 1].lexeme != ")" {
        return Err(parse_err("SC-PARSE-135", "danger call expected ')'."));
    }

    let mut args = Vec::new();
    let mut arg_start = open_index + 1;
    let mut depth = 0usize;
    let mut i = open_index + 1;
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

fn parse_timed_wait_expression(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Result<Option<(String, Expression)>, String> {
    if start >= end
        || tokens[start].kind() != TokenKind::Identifier
        || tokens[start].lexeme != "wait"
    {
        return Ok(None);
    }
    if start + 2 >= end
        || tokens[start + 1].kind() != TokenKind::Identifier
        || tokens[start + 2].lexeme != "for"
    {
        return Ok(None);
    }
    if start + 3 >= end {
        return Err(parse_err(
            "SC-PARSE-222",
            "timed wait expected Duration expression after 'for'.",
        ));
    }
    Ok(Some((
        tokens[start + 1].lexeme.clone(),
        parse_expression_range(tokens, start + 3, end)?,
    )))
}

pub fn parse_identifier_led_statement(
    tokens: &[Token],
    start_index: usize,
) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    let mut line_end = start_index;
    let mut depth_curly = 0usize;
    let mut depth_round = 0usize;
    let mut depth_square = 0usize;
    while line_end < tokens.len() {
        let lx = tokens[line_end].lexeme.as_str();
        if lx == "{" {
            depth_curly += 1;
        } else if lx == "}" {
            if depth_curly > 0 {
                depth_curly -= 1;
            } else if depth_round == 0 && depth_square == 0 {
                break;
            }
        } else if lx == "(" {
            depth_round += 1;
        } else if lx == ")" {
            depth_round = depth_round.saturating_sub(1);
        } else if lx == "[" {
            depth_square += 1;
        } else if lx == "]" {
            depth_square = depth_square.saturating_sub(1);
        }

        if tokens[line_end].kind() == TokenKind::NewLine
            && depth_curly == 0
            && depth_round == 0
            && depth_square == 0
        {
            break;
        }
        line_end += 1;
    }

    let on_idx = (start_index..line_end).find(|&i| {
        tokens[i].kind() == TokenKind::KeywordOnError
            && i + 1 < line_end
            && tokens[i + 1].lexeme == "error"
    });

    if on_idx.is_none()
        && start_index + 1 < line_end
        && tokens[start_index].kind() == TokenKind::Identifier
        && (tokens[start_index].lexeme == "wait" || tokens[start_index].lexeme == "run")
    {
        let expr = parse_expression_range(tokens, start_index, line_end)?;
        return Ok((
            Statement::ExpressionStatement {
                expr: Box::new(expr),
                loc,
            },
            line_end - start_index,
        ));
    }

    if on_idx.is_none()
        && start_index + 1 < line_end
        && tokens[start_index].kind() == TokenKind::Identifier
        && tokens[start_index].lexeme == "stop"
        && tokens[start_index + 1].kind() == TokenKind::Identifier
    {
        return Ok((
            Statement::StopTask {
                task_name: tokens[start_index + 1].lexeme.clone(),
                loc,
            },
            line_end - start_index,
        ));
    }

    if on_idx.is_none()
        && start_index + 4 < line_end
        && tokens[start_index].kind() == TokenKind::Identifier
        && tokens[start_index + 1].lexeme == "."
        && tokens[start_index + 2].kind() == TokenKind::Identifier
        && tokens[start_index + 2].lexeme == "clear"
        && tokens[start_index + 3].lexeme == "("
        && tokens[start_index + 4].lexeme == ")"
    {
        return Ok((
            Statement::MemoryClear {
                memory_name: tokens[start_index].lexeme.clone(),
                loc,
            },
            line_end - start_index,
        ));
    }

    if on_idx.is_none()
        && start_index + 1 < line_end
        && tokens[start_index].kind() == TokenKind::Identifier
        && tokens[start_index + 1].kind() == TokenKind::OpIncDec
    {
        let op = tokens[start_index + 1].lexeme.as_str();
        return Ok((
            Statement::IncDec {
                target: tokens[start_index].lexeme.clone(),
                is_increment: op == "++",
                loc,
            },
            line_end - start_index,
        ));
    }

    if on_idx.is_none()
        && start_index + 5 < line_end
        && (tokens[start_index].kind() == TokenKind::Identifier
            || tokens[start_index].kind() == TokenKind::KeywordMy)
        && tokens[start_index + 1].lexeme == "."
        && tokens[start_index + 2].kind() == TokenKind::Identifier
        && tokens[start_index + 2].lexeme == "push"
        && tokens[start_index + 3].lexeme == "("
        && tokens[line_end - 1].lexeme == ")"
    {
        let value = parse_expression_range(tokens, start_index + 4, line_end - 1)?;
        return Ok((
            Statement::ListPush {
                list_name: tokens[start_index].lexeme.clone(),
                value: Box::new(value),
                loc,
            },
            line_end - start_index,
        ));
    }

    if let Some(on_idx) = on_idx {
        let block_open = on_idx + 2;
        if block_open >= tokens.len() || tokens[block_open].lexeme != "{" {
            return Err(parse_err("SC-PARSE-136", "on error expected '{'."));
        }
        let block_end = find_block_end(tokens, block_open)?;
        let on_error_statements = parse_statements_range(tokens, block_open + 1, block_end)?;

        if start_index + 2 < on_idx
            && tokens[start_index].kind() == TokenKind::Identifier
            && tokens[start_index + 1].kind() == TokenKind::OpAssignment
            && tokens[start_index + 1].lexeme == "="
            && let Some((task_name, timeout)) =
                parse_timed_wait_expression(tokens, start_index + 2, on_idx)?
        {
            return Ok((
                Statement::DangerAssignOnError {
                    target: tokens[start_index].lexeme.clone(),
                    call_name: "__task_wait_for".to_string(),
                    args: vec![Expression::VariableReference(task_name), timeout],
                    on_error: Box::new(BlockStatement {
                        statements: on_error_statements,
                    }),
                    loc,
                },
                block_end + 1 - start_index,
            ));
        }

        if let Some((task_name, timeout)) =
            parse_timed_wait_expression(tokens, start_index, on_idx)?
        {
            return Ok((
                Statement::DangerCallOnError {
                    call_name: "__task_wait_for".to_string(),
                    args: vec![Expression::VariableReference(task_name), timeout],
                    on_error: Box::new(BlockStatement {
                        statements: on_error_statements,
                    }),
                    loc,
                },
                block_end + 1 - start_index,
            ));
        }

        if start_index + 7 <= on_idx
            && tokens[start_index].kind() == TokenKind::Identifier
            && tokens[start_index + 1].kind() == TokenKind::OpAssignment
            && tokens[start_index + 1].lexeme == "="
            && tokens[start_index + 2].kind() == TokenKind::Identifier
            && tokens[start_index + 3].lexeme == "."
            && tokens[start_index + 4].kind() == TokenKind::Identifier
            && tokens[start_index + 4].lexeme == "pop"
            && tokens[start_index + 5].lexeme == "("
            && tokens[start_index + 6].lexeme == ")"
        {
            return Ok((
                Statement::ListPopOnError {
                    target: tokens[start_index].lexeme.clone(),
                    list_name: tokens[start_index + 2].lexeme.clone(),
                    on_error: Box::new(BlockStatement {
                        statements: on_error_statements,
                    }),
                    loc,
                },
                block_end + 1 - start_index,
            ));
        }

        if start_index + 2 < on_idx
            && tokens[start_index].kind() == TokenKind::Identifier
            && tokens[start_index + 1].kind() == TokenKind::OpAssignment
            && tokens[start_index + 1].lexeme == "="
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
                    loc,
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
                loc,
            },
            block_end + 1 - start_index,
        ));
    }

    if start_index < line_end
        && start_index + 3 < line_end
        && (tokens[start_index].kind() == TokenKind::Identifier
            || tokens[start_index].kind() == TokenKind::KeywordMy)
        && tokens[start_index + 1].lexeme == "."
        && tokens[start_index + 2].kind() == TokenKind::Identifier
        && tokens[start_index + 3].kind() == TokenKind::OpAssignment
    {
        let object = tokens[start_index].lexeme.clone();
        let field = tokens[start_index + 2].lexeme.clone();
        let mut value = parse_expression_range(tokens, start_index + 4, line_end)?;
        if tokens[start_index + 3].lexeme != "=" {
            let op = tokens[start_index + 3]
                .lexeme
                .strip_suffix('=')
                .ok_or_else(|| parse_err("SC-PARSE-131", "invalid assignment operator."))?;
            value = Expression::BinaryOp {
                op: op.to_string(),
                left: Box::new(Expression::MemberAccess {
                    base: object.clone(),
                    field: field.clone(),
                }),
                right: Some(Box::new(value)),
            };
        }
        return Ok((
            Statement::FieldAssignment {
                object,
                field,
                value: Box::new(value),
                loc,
            },
            line_end - start_index,
        ));
    }

    if start_index < line_end
        && ((start_index + 1 < line_end && tokens[start_index + 1].lexeme == "(")
            || (start_index + 3 < line_end
                && tokens[start_index + 1].lexeme == "."
                && tokens[start_index + 2].kind() == TokenKind::Identifier
                && tokens[start_index + 3].lexeme == "("))
    {
        let expr = parse_expression_range(tokens, start_index, line_end)?;
        return Ok((
            Statement::ExpressionStatement {
                expr: Box::new(expr),
                loc,
            },
            line_end - start_index,
        ));
    }

    parse_assignment_statement(tokens, start_index)
}

pub fn parse_new_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    parse_variable_declaration(tokens, start_index, false)
}

pub fn parse_constant_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    parse_variable_declaration(tokens, start_index, true)
}

fn parse_variable_declaration(
    tokens: &[Token],
    start_index: usize,
    is_constant: bool,
) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    let expected = if is_constant {
        TokenKind::KeywordConstant
    } else {
        TokenKind::KeywordNew
    };
    if start_index >= tokens.len() || tokens[start_index].kind() != expected {
        return Err(parse_err(
            "SC-PARSE-137",
            if is_constant {
                "expected 'constant' keyword."
            } else {
                "expected 'new' keyword."
            },
        ));
    }
    if start_index + 2 >= tokens.len() {
        return Err(parse_err(
            "SC-PARSE-138",
            "incomplete variable declaration after 'new'.",
        ));
    }
    let mut idx = start_index + 1;
    let mut declared_type: Option<String> = None;

    // Supports:
    // new x = 1
    // new Int x = 1
    // new i32 List xs = [1, 2, 3]
    if let Some((type_name, type_consumed)) = parse_qualified_identifier(tokens, idx)
        && idx + type_consumed + 1 < tokens.len()
        && tokens[idx + type_consumed].kind() == TokenKind::Identifier
        && tokens[idx + type_consumed + 1].kind() == TokenKind::OpAssignment
        && tokens[idx + type_consumed + 1].lexeme == "="
    {
        declared_type = Some(type_name);
        idx += type_consumed;
    }
    if let Some((elem_type, type_consumed)) = parse_qualified_identifier(tokens, idx)
        && idx + type_consumed + 2 < tokens.len()
        && tokens[idx + type_consumed].kind() == TokenKind::Identifier
        && tokens[idx + type_consumed].lexeme == "List"
        && tokens[idx + type_consumed + 1].kind() == TokenKind::Identifier
        && tokens[idx + type_consumed + 2].kind() == TokenKind::OpAssignment
        && tokens[idx + type_consumed + 2].lexeme == "="
    {
        declared_type = Some(format!("{} List", elem_type));
        idx += type_consumed + 1;
    }

    if idx >= tokens.len() || tokens[idx].kind() != TokenKind::Identifier {
        return Err(parse_err(
            "SC-PARSE-139",
            "variable declaration expected identifier after 'new'.",
        ));
    }
    if idx + 1 >= tokens.len()
        || tokens[idx + 1].kind() != TokenKind::OpAssignment
        || tokens[idx + 1].lexeme != "="
    {
        return Err(parse_err(
            "SC-PARSE-140",
            "variable declaration expected '=' after identifier.",
        ));
    }

    let name = tokens[idx].lexeme.clone();
    let expr_start = idx + 2;
    let mut cursor = expr_start;
    let mut on_error_index = None;
    let mut depth_curly = 0usize;
    let mut depth_round = 0usize;
    let mut depth_square = 0usize;
    while cursor < tokens.len() {
        let lx = tokens[cursor].lexeme.as_str();
        if lx == "{" {
            depth_curly += 1;
        } else if lx == "}" {
            if depth_curly > 0 {
                depth_curly -= 1;
            } else if depth_round == 0 && depth_square == 0 {
                break;
            }
        } else if lx == "(" {
            depth_round += 1;
        } else if lx == ")" {
            depth_round = depth_round.saturating_sub(1);
        } else if lx == "[" {
            depth_square += 1;
        } else if lx == "]" {
            depth_square = depth_square.saturating_sub(1);
        }
        if depth_curly == 0
            && depth_round == 0
            && depth_square == 0
            && tokens[cursor].kind() == TokenKind::KeywordOnError
            && tokens.get(cursor + 1).map(|token| token.lexeme.as_str()) == Some("error")
        {
            on_error_index = Some(cursor);
            break;
        }
        if tokens[cursor].kind() == TokenKind::NewLine
            && depth_curly == 0
            && depth_round == 0
            && depth_square == 0
        {
            break;
        }
        cursor += 1;
    }
    let value = parse_expression_range(tokens, expr_start, cursor)?;
    let (on_error, consumed_end) = if let Some(on_index) = on_error_index {
        let block_open = on_index + 2;
        if tokens.get(block_open).map(|token| token.lexeme.as_str()) != Some("{") {
            return Err(parse_err(
                "SC-PARSE-234",
                "fallible declaration expected '{' after 'on error'.",
            ));
        }
        let block_end = find_block_end(tokens, block_open)?;
        let statements = parse_statements_range(tokens, block_open + 1, block_end)?;
        (Some(Box::new(BlockStatement { statements })), block_end + 1)
    } else {
        (None, cursor)
    };
    Ok((
        Statement::VarDecl {
            name,
            value: Box::new(value),
            is_constant,
            declared_type,
            on_error,
            loc,
        },
        (consumed_end - start_index).max(4),
    ))
}

pub fn parse_memory_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index >= tokens.len()
        || tokens[start_index].kind() != TokenKind::Identifier
        || tokens[start_index].lexeme != "Memory"
    {
        return Err(parse_err(
            "SC-PARSE-159",
            "expected 'Memory' declaration start.",
        ));
    }
    if start_index + 4 >= tokens.len() {
        return Err(parse_err("SC-PARSE-160", "incomplete Memory declaration."));
    }
    if tokens[start_index + 1].kind() != TokenKind::Identifier {
        return Err(parse_err(
            "SC-PARSE-161",
            "Memory declaration expected identifier name.",
        ));
    }
    if tokens[start_index + 2].kind() != TokenKind::OpAssignment
        || tokens[start_index + 2].lexeme != "="
    {
        return Err(parse_err(
            "SC-PARSE-162",
            "Memory declaration expected '=' after name.",
        ));
    }
    if tokens[start_index + 3].kind() != TokenKind::Identifier
        || tokens[start_index + 3].lexeme != "memory"
    {
        return Err(parse_err(
            "SC-PARSE-163",
            "Memory declaration expected memory(...) initializer.",
        ));
    }
    let mut kind = MemoryKind::Dynamic;
    let mut open_paren = start_index + 4;
    if tokens.get(open_paren).map(|token| token.lexeme.as_str()) == Some(".") {
        let Some(kind_token) = tokens.get(open_paren + 1) else {
            return Err(parse_err(
                "SC-PARSE-164",
                "Memory declaration expected 'child' or 'static' after 'memory.'.",
            ));
        };
        kind = match kind_token.lexeme.as_str() {
            "child" => MemoryKind::Child,
            "static" => MemoryKind::Static,
            other => {
                return Err(parse_err(
                    "SC-PARSE-164",
                    format!("unsupported Memory initializer 'memory.{other}'."),
                ));
            }
        };
        open_paren += 2;
    }
    if tokens.get(open_paren).map(|token| token.lexeme.as_str()) != Some("(") {
        return Err(parse_err(
            "SC-PARSE-164",
            "Memory declaration expected '(' after memory initializer.",
        ));
    }

    let args_start = open_paren + 1;
    let mut cursor = args_start;
    let mut depth = 1usize;
    let mut first_comma = None;
    while cursor < tokens.len() && depth > 0 {
        if tokens[cursor].lexeme == "(" {
            depth += 1;
        } else if tokens[cursor].lexeme == ")" {
            depth -= 1;
        } else if tokens[cursor].lexeme == "," && depth == 1 && first_comma.is_none() {
            first_comma = Some(cursor);
        }
        cursor += 1;
    }
    if depth != 0 {
        return Err(parse_err(
            "SC-PARSE-165",
            "Memory declaration expected ')' after size.",
        ));
    }
    let close_paren = cursor - 1;
    let size_end = first_comma.unwrap_or(close_paren);
    if args_start == size_end {
        return Err(parse_err(
            "SC-PARSE-166",
            "Memory declaration expected non-empty size inside memory(...).",
        ));
    }
    let size = super::expressions::parse_memory_size_expression(tokens, args_start, size_end)?;

    let mut allow_grow = false;
    let mut allow_drop = false;
    let mut policy_cursor = first_comma.map(|index| index + 1).unwrap_or(close_paren);
    while policy_cursor < close_paren {
        if tokens[policy_cursor].lexeme == "," {
            policy_cursor += 1;
            continue;
        }
        if policy_cursor + 1 >= close_paren
            || tokens[policy_cursor].lexeme != "allow"
            || !matches!(tokens[policy_cursor + 1].lexeme.as_str(), "grow" | "drop")
        {
            return Err(parse_err(
                "SC-PARSE-166",
                "Memory policy expected 'allow grow' or 'allow drop'.",
            ));
        }
        match tokens[policy_cursor + 1].lexeme.as_str() {
            "grow" if allow_grow => {
                return Err(parse_err(
                    "SC-PARSE-166",
                    "duplicate Memory policy 'allow grow'.",
                ));
            }
            "grow" => allow_grow = true,
            "drop" if allow_drop => {
                return Err(parse_err(
                    "SC-PARSE-166",
                    "duplicate Memory policy 'allow drop'.",
                ));
            }
            "drop" => allow_drop = true,
            _ => unreachable!(),
        }
        policy_cursor += 2;
        if policy_cursor < close_paren && tokens[policy_cursor].lexeme != "," {
            return Err(parse_err(
                "SC-PARSE-166",
                "Memory policies must be separated by commas.",
            ));
        }
    }

    let mut consumed_end = close_paren + 1;
    let mut on_error = None;
    if consumed_end + 2 < tokens.len()
        && tokens[consumed_end].kind() == TokenKind::KeywordOnError
        && tokens[consumed_end + 1].lexeme == "error"
    {
        if tokens[consumed_end + 2].lexeme != "{" {
            return Err(parse_err(
                "SC-PARSE-167",
                "Memory declaration on error expected '{'.",
            ));
        }
        let (block, block_end) = parse_braced_block(tokens, consumed_end + 2)?;
        on_error = Some(block);
        consumed_end = block_end + 1;
    }

    Ok((
        Statement::MemoryDecl {
            name: tokens[start_index + 1].lexeme.clone(),
            size: Box::new(size),
            kind,
            allow_grow,
            allow_drop,
            on_error,
            loc,
        },
        consumed_end - start_index,
    ))
}

pub fn parse_place_in_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index >= tokens.len()
        || tokens[start_index].kind() != TokenKind::Identifier
        || tokens[start_index].lexeme != "place"
    {
        return Err(parse_err("SC-PARSE-168", "expected 'place' keyword."));
    }
    if start_index + 2 >= tokens.len() {
        return Err(parse_err("SC-PARSE-169", "incomplete place in statement."));
    }
    if tokens[start_index + 1].kind() != TokenKind::KeywordIn {
        return Err(parse_err(
            "SC-PARSE-170",
            "place statement expected 'in' after place.",
        ));
    }
    if tokens[start_index + 2].kind() != TokenKind::Identifier {
        return Err(parse_err(
            "SC-PARSE-171",
            "place in expected memory identifier.",
        ));
    }

    let memory_name = tokens[start_index + 2].lexeme.clone();
    let mut cursor = start_index + 3;
    if cursor + 1 < tokens.len()
        && tokens[cursor].kind() == TokenKind::KeywordOnError
        && tokens[cursor + 1].lexeme == "error"
    {
        return Err(parse_err(
            "SC-PARSE-172",
            "legacy placement syntax removed: use 'place in memory { ... } on error { ... }'.",
        ));
    }

    if cursor >= tokens.len() || tokens[cursor].lexeme != "{" {
        return Err(parse_err(
            "SC-PARSE-173",
            "place in expected body block '{ ... }'.",
        ));
    }
    let (body, body_end) = parse_braced_block(tokens, cursor)?;
    cursor = body_end + 1;

    let mut on_error = None;
    if cursor + 2 < tokens.len()
        && tokens[cursor].kind() == TokenKind::KeywordOnError
        && tokens[cursor + 1].lexeme == "error"
    {
        if tokens[cursor + 2].lexeme != "{" {
            return Err(parse_err(
                "SC-PARSE-174",
                "place in trailing on error expected '{'.",
            ));
        }
        let (block, block_end) = parse_braced_block(tokens, cursor + 2)?;
        on_error = Some(block);
        cursor = block_end + 1;
    }

    Ok((
        Statement::PlaceIn {
            memory_name,
            on_error,
            body,
            loc,
        },
        cursor - start_index,
    ))
}

pub fn parse_label_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    parse_label_declaration_inner(tokens, start_index, false)
}

fn parse_label_declaration_inner(
    tokens: &[Token],
    start_index: usize,
    is_local: bool,
) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordLabel {
        return Err(parse_err("SC-PARSE-141", "expected 'label' keyword."));
    }
    if start_index + 1 >= tokens.len() || tokens[start_index + 1].kind() != TokenKind::Identifier {
        return Err(parse_err(
            "SC-PARSE-142",
            "label declaration expected identifier name.",
        ));
    }
    let open = start_index + 2;
    if open >= tokens.len() || tokens[open].lexeme != "{" {
        return Err(parse_err("SC-PARSE-143", "label declaration expected '{'."));
    }
    let close = find_block_end(tokens, open)?;
    let mut variants = Vec::new();
    let mut cursor = open + 1;
    while cursor < close {
        if tokens[cursor].kind() == TokenKind::NewLine {
            cursor += 1;
            continue;
        }
        if tokens[cursor].kind() != TokenKind::Identifier {
            return Err(parse_err(
                "SC-PARSE-143",
                "label variant expected identifier name.",
            ));
        }
        let name = tokens[cursor].lexeme.clone();
        if cursor + 2 >= close
            || tokens[cursor + 1].kind() != TokenKind::OpAssignment
            || tokens[cursor + 1].lexeme != "="
            || tokens[cursor + 2].kind() != TokenKind::TypeInt
        {
            return Err(parse_err(
                "SC-PARSE-143",
                format!(
                    "label variant '{}' requires explicit integer discriminant.",
                    name
                ),
            ));
        }
        let discriminant = tokens[cursor + 2].lexeme.parse::<i64>().map_err(|_| {
            parse_err(
                "SC-PARSE-143",
                format!("label variant '{}' has invalid integer discriminant.", name),
            )
        })?;
        variants.push(LabelVariant { name, discriminant });
        cursor += 3;
    }
    Ok((
        Statement::LabelDecl {
            name: tokens[start_index + 1].lexeme.clone(),
            variants,
            is_local,
            loc,
        },
        close + 1 - start_index,
    ))
}

pub fn parse_tag_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    parse_tag_declaration_inner(tokens, start_index, false)
}

fn parse_tag_declaration_inner(
    tokens: &[Token],
    start_index: usize,
    is_local: bool,
) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if tokens.get(start_index).map(Token::kind) != Some(TokenKind::KeywordTag) {
        return Err(parse_err("SC-PARSE-163", "expected 'tag' keyword."));
    }
    if tokens.get(start_index + 1).map(Token::kind) != Some(TokenKind::Identifier) {
        return Err(parse_err(
            "SC-PARSE-164",
            "tag declaration expected identifier name.",
        ));
    }
    let open = start_index + 2;
    if tokens.get(open).map(|token| token.lexeme.as_str()) != Some("{") {
        return Err(parse_err("SC-PARSE-165", "tag declaration expected '{'."));
    }
    let close = find_block_end(tokens, open)?;
    let mut variants = Vec::new();
    for token in &tokens[open + 1..close] {
        if token.kind() == TokenKind::Identifier {
            variants.push(token.lexeme.clone());
        } else if token.kind() != TokenKind::NewLine {
            return Err(parse_err(
                "SC-PARSE-166",
                "tag body accepts symbolic variant names only.",
            ));
        }
    }
    Ok((
        Statement::TagDecl {
            name: tokens[start_index + 1].lexeme.clone(),
            variants,
            is_local,
            loc,
        },
        close + 1 - start_index,
    ))
}

pub fn parse_struct_declaration(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    parse_struct_declaration_inner(tokens, start_index, false, false)
}

fn parse_struct_declaration_inner(
    tokens: &[Token],
    start_index: usize,
    is_local: bool,
    is_external: bool,
) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordStruct {
        return Err(parse_err("SC-PARSE-144", "expected 'struct' keyword."));
    }
    if start_index + 1 >= tokens.len() || tokens[start_index + 1].kind() != TokenKind::Identifier {
        return Err(parse_err(
            "SC-PARSE-145",
            "struct declaration expected identifier name.",
        ));
    }
    let open = start_index + 2;
    if open >= tokens.len() || tokens[open].lexeme != "{" {
        return Err(parse_err(
            "SC-PARSE-146",
            "struct declaration expected '{'.",
        ));
    }
    let close = find_block_end(tokens, open)?;
    let mut fields: Vec<StructField> = Vec::new();
    let mut methods: Vec<StructMethod> = Vec::new();
    let mut cursor = open + 1;
    while cursor < close {
        if tokens[cursor].kind() == TokenKind::NewLine {
            cursor += 1;
            continue;
        }
        let mut field_hidden = false;
        if tokens[cursor].kind() == TokenKind::KeywordHide
            || (tokens[cursor].kind() == TokenKind::Identifier && tokens[cursor].lexeme == "hide")
        {
            field_hidden = true;
            cursor += 1;
        }
        let method_start = if tokens[cursor].kind() == TokenKind::Identifier
            && tokens[cursor].lexeme == "danger"
        {
            if cursor + 1 < close && tokens[cursor + 1].kind() == TokenKind::KeywordFn {
                Some(cursor)
            } else {
                None
            }
        } else if tokens[cursor].kind() == TokenKind::KeywordFn {
            Some(cursor)
        } else {
            None
        };
        if let Some(ms) = method_start {
            if is_external {
                return Err(parse_err(
                    "SC-PARSE-231",
                    "external struct cannot declare methods; wrap behavior in Skadi functions.",
                ));
            }
            let (method, consumed) = parse_struct_method(tokens, ms, close)?;
            methods.push(method);
            cursor = ms + consumed;
            continue;
        }
        if let Some((field_type, name_index)) = parse_type_name_at(tokens, cursor)
            && name_index < close
            && tokens[name_index].kind() == TokenKind::Identifier
        {
            let first_name = tokens[name_index].lexeme.clone();
            fields.push(StructField {
                field_type: field_type.clone(),
                name: first_name,
                is_hidden: field_hidden,
            });
            cursor = name_index + 1;
            while cursor + 1 < close
                && tokens[cursor].lexeme == ","
                && tokens[cursor + 1].kind() == TokenKind::Identifier
            {
                fields.push(StructField {
                    field_type: field_type.clone(),
                    name: tokens[cursor + 1].lexeme.clone(),
                    is_hidden: field_hidden,
                });
                cursor += 2;
            }
            continue;
        }
        cursor += 1;
    }
    Ok((
        Statement::StructDecl {
            name: tokens[start_index + 1].lexeme.clone(),
            fields,
            methods,
            is_local,
            is_external,
            is_resource: false,
            loc,
        },
        close + 1 - start_index,
    ))
}

fn parse_struct_method(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Result<(StructMethod, usize), String> {
    let mut current_index = start;
    let mut is_danger = false;
    if current_index < end
        && tokens[current_index].kind() == TokenKind::Identifier
        && tokens[current_index].lexeme == "danger"
    {
        is_danger = true;
        current_index += 1;
    }
    if current_index >= end || tokens[current_index].kind() != TokenKind::KeywordFn {
        return Err(parse_err("SC-PARSE-153", "expected 'fn' in struct method."));
    }
    if current_index + 1 >= end || tokens[current_index + 1].kind() != TokenKind::Identifier {
        return Err(parse_err("SC-PARSE-154", "struct method expected name."));
    }
    let name = tokens[current_index + 1].lexeme.clone();
    if current_index + 2 >= end || tokens[current_index + 2].lexeme != "(" {
        return Err(parse_err(
            "SC-PARSE-155",
            "struct method expected '(' after name.",
        ));
    }
    current_index += 3;
    let mut params = Vec::new();
    while current_index < end && tokens[current_index].lexeme != ")" {
        if tokens[current_index].lexeme == "," {
            current_index += 1;
            continue;
        }
        let mut borrow = BorrowMode::Value;
        let mut type_index = current_index;
        if tokens[type_index].kind() == TokenKind::KeywordEdit {
            borrow = BorrowMode::EditMutable;
            type_index += 1;
        } else if tokens[type_index].kind() == TokenKind::KeywordView {
            borrow = BorrowMode::View;
            type_index += 1;
        } else if tokens[type_index].kind() == TokenKind::KeywordMove {
            borrow = BorrowMode::Move;
            type_index += 1;
        }
        if let Some((param_type, name_index)) = parse_type_name_at(tokens, type_index)
            && name_index < end
            && tokens[name_index].kind() == TokenKind::Identifier
        {
            params.push(FunctionParam {
                param_type: Some(param_type),
                name: tokens[name_index].lexeme.clone(),
                borrow,
            });
            current_index = name_index + 1;
            continue;
        }
        if tokens[current_index].kind() == TokenKind::Identifier {
            params.push(FunctionParam {
                param_type: None,
                name: tokens[current_index].lexeme.clone(),
                borrow: BorrowMode::Value,
            });
        }
        current_index += 1;
    }
    if current_index >= end || tokens[current_index].lexeme != ")" {
        return Err(parse_err(
            "SC-PARSE-156",
            "struct method expected ')' after parameters.",
        ));
    }
    current_index += 1;
    let mut returns = None;
    let mut uses_returns_keyword = false;
    if current_index < end && tokens[current_index].lexeme == "returns" {
        uses_returns_keyword = true;
        current_index += 1;
        if let Some((return_type, next_index)) = parse_type_name_at(tokens, current_index) {
            returns = Some(return_type);
            current_index = next_index;
        } else {
            return Err(parse_err(
                "SC-PARSE-163",
                "expected return type after 'returns'.",
            ));
        }
    } else if let Some((return_type, next_index)) = parse_type_name_at(tokens, current_index)
        && next_index < end
        && tokens[next_index].lexeme == "{"
    {
        returns = Some(return_type);
        current_index = next_index;
    }
    if current_index >= end || tokens[current_index].lexeme != "{" {
        return Err(parse_err(
            "SC-PARSE-157",
            "struct method expected '{' body.",
        ));
    }
    let block_end = find_block_end(tokens, current_index)?;
    let body_stmts = parse_statements_range(tokens, current_index + 1, block_end)?;
    let consumed = block_end + 1 - start;
    Ok((
        StructMethod {
            name,
            params,
            body: Box::new(BlockStatement {
                statements: body_stmts,
            }),
            returns,
            uses_returns_keyword,
            is_danger,
        },
        consumed,
    ))
}

pub fn parse_on_block_statement(tokens: &[Token], start_index: usize) -> ParseResult<Statement> {
    let loc = Location {
        line: tokens[start_index].line,
        column: tokens[start_index].col,
    };
    if start_index >= tokens.len() || tokens[start_index].kind() != TokenKind::KeywordOnError {
        return Err(parse_err("SC-PARSE-147", "expected 'on' keyword."));
    }
    let mut cursor = start_index + 1;
    while cursor < tokens.len() && tokens[cursor].lexeme != "{" {
        cursor += 1;
    }
    if cursor >= tokens.len() {
        return Err(parse_err("SC-PARSE-148", "on-block expected '{'."));
    }
    let close = find_block_end(tokens, cursor)?;
    let trigger = if start_index + 1 < tokens.len() {
        tokens[start_index + 1].lexeme.clone()
    } else {
        "unknown".to_string()
    };
    let target = if start_index + 2 < cursor {
        Some(render_token_slice(tokens, start_index + 2, cursor))
    } else {
        None
    };
    let body_statements = parse_statements_range(tokens, cursor + 1, close)?;
    Ok((
        Statement::OnBlock {
            trigger,
            target,
            body: Box::new(BlockStatement {
                statements: body_statements,
            }),
            loc,
        },
        close + 1 - start_index,
    ))
}
