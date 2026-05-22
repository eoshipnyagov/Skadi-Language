use std::collections::HashMap;

use crate::ast_nodes::{BlockStatement, Expression, FunctionParam, Program, Statement};
use crate::diagnostics::{format_diagnostic, DiagnosticKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueType {
    Int,
    Float,
    Bool,
    Unknown,
}

#[derive(Clone, Debug)]
struct FunctionSig {
    is_danger: bool,
    return_type: Option<ValueType>,
    param_types: Vec<ValueType>,
}

#[derive(Clone, Copy)]
struct FnContext {
    is_danger: bool,
    return_type: Option<ValueType>,
}

fn statement_loc(stmt: &Statement) -> Option<(u32, u32)> {
    match stmt {
        Statement::VarDecl { loc, .. }
        | Statement::Assignment { loc, .. }
        | Statement::FunctionDef { loc, .. }
        | Statement::IfStatement { loc, .. }
        | Statement::ForLoop { loc, .. }
        | Statement::WhenBlock { loc, .. }
        | Statement::WhileLoop { loc, .. }
        | Statement::LoopStatement { loc, .. }
        | Statement::LabelDecl { loc, .. }
        | Statement::StructDecl { loc, .. }
        | Statement::OnBlock { loc, .. }
        | Statement::DangerAssignOnError { loc, .. }
        | Statement::DangerCallOnError { loc, .. }
        | Statement::ReturnError { loc, .. }
        | Statement::ReturnStatement { loc, .. }
        | Statement::BlockStatement { loc, .. }
        | Statement::OnErrorBlock { loc, .. } => Some((loc.line, loc.column)),
    }
}

fn err_at(stmt: &Statement, msg: String) -> String {
    if let Some((line, col)) = statement_loc(stmt) {
        format_diagnostic(
            DiagnosticKind::Semantic,
            Some("SC-SEM-001"),
            msg,
            Some(line),
            Some(col),
            None,
        )
    } else {
        format_diagnostic(
            DiagnosticKind::Semantic,
            Some("SC-SEM-001"),
            msg,
            None,
            None,
            None,
        )
    }
}

pub fn semantic_analyze(program: &Program) -> Result<(), String> {
    let mut functions: HashMap<String, FunctionSig> = HashMap::new();
    let mut labels: HashMap<String, Vec<String>> = HashMap::new();

    for stmt in &program.statements {
        if let Statement::FunctionDef {
            name,
            is_danger,
            returns,
            params,
            ..
        } = stmt
        {
            functions.insert(
                name.clone(),
                FunctionSig {
                    is_danger: *is_danger,
                    return_type: returns.as_deref().map(parse_type_name).or(Some(ValueType::Int)),
                    param_types: params.iter().map(param_type_or_default).collect(),
                },
            );
        }
        if let Statement::LabelDecl { name, variants, .. } = stmt {
            labels.insert(name.clone(), variants.clone());
        }
    }

    validate_error_code_label(&labels)?;

    let mut scope: HashMap<String, ValueType> = HashMap::new();
    analyze_statements(&program.statements, &mut scope, &functions, &labels, None)
}

fn validate_error_code_label(labels: &HashMap<String, Vec<String>>) -> Result<(), String> {
    if let Some(error_codes) = labels.get("ErrorCode") {
        if error_codes.is_empty() {
            return Err("label ErrorCode must define at least one variant.".to_string());
        }
        if error_codes[0] != "Ok" {
            return Err("label ErrorCode must start with 'Ok' variant.".to_string());
        }
    }
    Ok(())
}

fn parse_type_name(name: &str) -> ValueType {
    match name {
        "Int" | "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" => ValueType::Int,
        "Float" | "f64" | "f32" => ValueType::Float,
        "bool" => ValueType::Bool,
        _ => ValueType::Unknown,
    }
}

fn can_assign(target: ValueType, source: ValueType) -> bool {
    target == source || (target == ValueType::Float && source == ValueType::Int)
}

fn validate_call_args(
    name: &str,
    args: &[Expression],
    sig: &FunctionSig,
    scope: &HashMap<String, ValueType>,
    functions: &HashMap<String, FunctionSig>,
) -> Result<(), String> {
    if args.len() != sig.param_types.len() {
        return Err(format!(
            "argument count mismatch for '{}': expected {}, got {}.",
            name,
            sig.param_types.len(),
            args.len()
        ));
    }
    for (arg, expected_ty) in args.iter().zip(sig.param_types.iter().copied()) {
        let actual_ty = infer_expression_type(arg, scope, functions)?;
        if !can_assign(expected_ty, actual_ty) {
            return Err(format!(
                "argument type mismatch for '{}': expected {:?}, got {:?}.",
                name, expected_ty, actual_ty
            ));
        }
    }
    Ok(())
}

fn analyze_statements(
    statements: &[Statement],
    scope: &mut HashMap<String, ValueType>,
    functions: &HashMap<String, FunctionSig>,
    labels: &HashMap<String, Vec<String>>,
    fn_ctx: Option<FnContext>,
) -> Result<(), String> {
    for stmt in statements {
        analyze_statement(stmt, scope, functions, labels, fn_ctx)?;
    }
    Ok(())
}

fn analyze_statement(
    stmt: &Statement,
    scope: &mut HashMap<String, ValueType>,
    functions: &HashMap<String, FunctionSig>,
    labels: &HashMap<String, Vec<String>>,
    fn_ctx: Option<FnContext>,
) -> Result<(), String> {
    match stmt {
        Statement::VarDecl {
            name,
            value,
            declared_type,
            ..
        } => {
            if scope.contains_key(name) {
                return Err(format!(
                    "{}",
                    err_at(stmt, format!("redeclaration in same scope: '{}' is already defined.", name))
                ));
            }
            if contains_variable(value, name) {
                return Err(err_at(
                    stmt,
                    format!("invalid initialization: '{}' is used in its own initializing expression.", name),
                ));
            }
            let value_ty = infer_expression_type(value, scope, functions)?;
            let final_ty = if let Some(tn) = declared_type {
                let declared = parse_type_name(tn);
                if !can_assign(declared, value_ty) {
                    return Err(format!(
                        "{}",
                        err_at(
                            stmt,
                            format!(
                                "type mismatch in declaration '{}': cannot assign {:?} to {:?}.",
                                name, value_ty, declared
                            ),
                        )
                    ));
                }
                declared
            } else {
                value_ty
            };
            scope.insert(name.clone(), final_ty);
            Ok(())
        }
        Statement::Assignment { target, value, .. } => {
            let Some(target_ty) = scope.get(target).copied() else {
                return Err(format!(
                    "{}",
                    err_at(
                        stmt,
                        format!("use-before-definition: '{}' is not defined in current scope.", target),
                    )
                ));
            };
            let value_ty = infer_expression_type(value, scope, functions)?;
            if !can_assign(target_ty, value_ty) {
                return Err(format!(
                    "{}",
                    err_at(
                        stmt,
                        format!(
                            "type mismatch in assignment to '{}': cannot assign {:?} to {:?}.",
                            target, value_ty, target_ty
                        ),
                    )
                ));
            }
            Ok(())
        }
        Statement::FunctionDef {
            name, params, body, ..
        } => {
            let mut fn_scope = scope.clone();
            for p in params {
                let pty = param_type_or_default(p);
                fn_scope.insert(p.name.clone(), pty);
            }
            let Some(sig) = functions.get(name) else {
                return Err(format!("internal error: missing function signature for '{}'.", name));
            };
            let local_ctx = FnContext {
                is_danger: sig.is_danger,
                return_type: sig.return_type,
            };
            analyze_block(body, &mut fn_scope, functions, labels, Some(local_ctx))?;
            if sig.is_danger && !block_guarantees_termination(body) {
                return Err(format!(
                    "{}",
                    err_at(
                        stmt,
                        format!("danger fn '{}' must end with explicit return/return error on all paths.", name),
                    )
                ));
            }
            Ok(())
        }
        Statement::IfStatement {
            condition,
            then_block,
            else_block,
            ..
        } => {
            let cty = infer_expression_type(condition, scope, functions)?;
            if cty != ValueType::Bool {
                return Err(err_at(stmt, "if condition must be bool.".to_string()));
            }
            let mut then_scope = scope.clone();
            analyze_block(then_block, &mut then_scope, functions, labels, fn_ctx)?;
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                analyze_block(else_block, &mut else_scope, functions, labels, fn_ctx)?;
            }
            Ok(())
        }
        Statement::ForLoop {
            initialization,
            condition,
            update,
            body,
            ..
        } => {
            let mut loop_scope = scope.clone();
            if let Some(init) = initialization {
                if let Expression::VariableReference(name) = init.as_ref() {
                    loop_scope.insert(name.clone(), ValueType::Unknown);
                } else {
                    let _ = infer_expression_type(init, &loop_scope, functions)?;
                }
            }
            if let Some(cond) = condition {
                let _ = infer_expression_type(cond, &loop_scope, functions)?;
            }
            if let Some(upd) = update {
                let _ = infer_expression_type(upd, &loop_scope, functions)?;
            }
            analyze_block(body, &mut loop_scope, functions, labels, fn_ctx)
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            let when_ty = infer_expression_type(when_expression, scope, functions)?;
            for (case_exprs, block) in cases {
                for expr in case_exprs {
                    let case_ty = infer_expression_type(expr, scope, functions)?;
                    if !can_assign(when_ty, case_ty) && !can_assign(case_ty, when_ty) {
                        return Err(format!(
                            "type mismatch in when-case: case type {:?} incompatible with when type {:?}.",
                            case_ty, when_ty
                        ));
                    }
                }
                let mut case_scope = scope.clone();
                analyze_block(block, &mut case_scope, functions, labels, fn_ctx)?;
            }
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                analyze_block(else_block, &mut else_scope, functions, labels, fn_ctx)?;
            }
            Ok(())
        }
        Statement::WhileLoop { condition, body, .. } => {
            let cty = infer_expression_type(condition, scope, functions)?;
            if cty != ValueType::Bool {
                return Err(err_at(stmt, "while condition must be bool.".to_string()));
            }
            let mut while_scope = scope.clone();
            analyze_block(body, &mut while_scope, functions, labels, fn_ctx)
        }
        Statement::LoopStatement { body, .. } => {
            let mut local_scope = scope.clone();
            analyze_block(body, &mut local_scope, functions, labels, fn_ctx)
        }
        Statement::OnErrorBlock { statements, .. } | Statement::BlockStatement { statements, .. } => {
            let mut local_scope = scope.clone();
            analyze_statements(statements, &mut local_scope, functions, labels, fn_ctx)
        }
        Statement::OnBlock { trigger, .. } => {
            if trigger == "error" {
                return Err(
                    err_at(
                        stmt,
                        "unsupported context: 'on error { ... }' is not yet semantically bound to a danger call."
                            .to_string(),
                    ),
                );
            }
            Ok(())
        }
        Statement::DangerAssignOnError {
            target,
            call_name,
            args,
            on_error,
            ..
        } => {
            let Some(sig) = functions.get(call_name) else {
                return Err(format!("unknown function '{}' in on error call.", call_name));
            };
            if !sig.is_danger {
                return Err(format!(
                    "{}",
                    err_at(
                        stmt,
                        format!("on error requires danger fn call: '{}' is not declared as danger.", call_name),
                    )
                ));
            }
            if !scope.contains_key(target) {
                return Err(format!(
                    "{}",
                    err_at(
                        stmt,
                        format!("use-before-definition: '{}' is not defined in current scope.", target),
                    )
                ));
            }
            validate_call_args(call_name, args, sig, scope, functions)?;
            let mut on_error_scope = scope.clone();
            analyze_block(on_error, &mut on_error_scope, functions, labels, fn_ctx)
        }
        Statement::DangerCallOnError {
            call_name,
            args,
            on_error,
            ..
        } => {
            let Some(sig) = functions.get(call_name) else {
                return Err(format!("unknown function '{}' in on error call.", call_name));
            };
            if !sig.is_danger {
                return Err(format!(
                    "{}",
                    err_at(
                        stmt,
                        format!("on error requires danger fn call: '{}' is not declared as danger.", call_name),
                    )
                ));
            }
            validate_call_args(call_name, args, sig, scope, functions)?;
            let mut on_error_scope = scope.clone();
            analyze_block(on_error, &mut on_error_scope, functions, labels, fn_ctx)
        }
        Statement::ReturnError { code, .. } => {
            if fn_ctx.map(|c| c.is_danger) != Some(true) {
                return Err(err_at(stmt, "return error is allowed only inside danger fn.".to_string()));
            }
            let Some(error_codes) = labels.get("ErrorCode") else {
                return Err(err_at(stmt, "return error requires label ErrorCode declaration.".to_string()));
            };
            if !error_codes.iter().any(|v| v == code) {
                return Err(err_at(stmt, format!("unknown ErrorCode variant: '{}'.", code)));
            }
            Ok(())
        }
        Statement::ReturnStatement { value, .. } => {
            if let Some(ctx) = fn_ctx {
                if let Some(expr) = value {
                    let actual = infer_expression_type(expr, scope, functions)?;
                    if let Some(expected) = ctx.return_type {
                        if !can_assign(expected, actual) {
                            return Err(format!(
                                "type mismatch in return: cannot return {:?} where {:?} expected.",
                                actual, expected
                            ));
                        }
                    }
                } else if !ctx.is_danger && ctx.return_type.is_some() {
                    return Err(err_at(
                        stmt,
                        "non-danger function with return type must return a value.".to_string(),
                    ));
                }
            } else if let Some(expr) = value {
                let _ = infer_expression_type(expr, scope, functions)?;
            }
            Ok(())
        }
        Statement::LabelDecl { .. } | Statement::StructDecl { .. } => Ok(()),
    }
}

fn analyze_block(
    block: &BlockStatement,
    scope: &mut HashMap<String, ValueType>,
    functions: &HashMap<String, FunctionSig>,
    labels: &HashMap<String, Vec<String>>,
    fn_ctx: Option<FnContext>,
) -> Result<(), String> {
    analyze_statements(&block.statements, scope, functions, labels, fn_ctx)
}

fn param_type_or_default(param: &FunctionParam) -> ValueType {
    param
        .param_type
        .as_deref()
        .map(parse_type_name)
        .unwrap_or(ValueType::Int)
}

fn infer_expression_type(
    expr: &Expression,
    scope: &HashMap<String, ValueType>,
    functions: &HashMap<String, FunctionSig>,
) -> Result<ValueType, String> {
    match expr {
        Expression::LiteralInt(_) => Ok(ValueType::Int),
        Expression::LiteralFloat(_) => Ok(ValueType::Float),
        Expression::LiteralBool(_) => Ok(ValueType::Bool),
        Expression::VariableReference(name) => scope
            .get(name)
            .copied()
            .ok_or_else(|| format!("use-before-definition: '{}' is not defined in current scope.", name)),
        Expression::Call { name, args } => {
            let Some(sig) = functions.get(name) else {
                return Err(format!("unknown function '{}' in expression call.", name));
            };
            validate_call_args(name, args, sig, scope, functions)?;
            Ok(sig.return_type.unwrap_or(ValueType::Unknown))
        }
        Expression::BinaryOp { op, left, right } => {
            if op == "neg" {
                let lt = infer_expression_type(left, scope, functions)?;
                if lt == ValueType::Int || lt == ValueType::Float {
                    return Ok(lt);
                }
                return Err("unary '-' requires numeric operand.".to_string());
            }
            if op == "not" {
                let lt = infer_expression_type(left, scope, functions)?;
                if lt == ValueType::Bool || lt == ValueType::Int {
                    return Ok(ValueType::Bool);
                }
                return Err("unary 'not' requires bool/int operand.".to_string());
            }
            let lt = infer_expression_type(left, scope, functions)?;
            let rt = if let Some(r) = right {
                infer_expression_type(r, scope, functions)?
            } else {
                ValueType::Unknown
            };
            match op.as_str() {
                "+" | "-" | "*" | "/" | "div" | "mod" | "^" => {
                    if (lt == ValueType::Int || lt == ValueType::Float)
                        && (rt == ValueType::Int || rt == ValueType::Float)
                    {
                        if lt == ValueType::Float || rt == ValueType::Float {
                            Ok(ValueType::Float)
                        } else {
                            Ok(ValueType::Int)
                        }
                    } else {
                        Err(format!("operator '{}' requires numeric operands.", op))
                    }
                }
                "==" | "!=" | "<" | ">" | "<=" | ">=" => Ok(ValueType::Bool),
                "and" | "or" | "xor" => {
                    if (lt == ValueType::Bool || lt == ValueType::Int)
                        && (rt == ValueType::Bool || rt == ValueType::Int)
                    {
                        Ok(ValueType::Bool)
                    } else {
                        Err(format!("operator '{}' requires bool/int operands.", op))
                    }
                }
                _ => Ok(ValueType::Unknown),
            }
        }
        Expression::StructConstruction { fields } => {
            for value in fields.values() {
                let _ = infer_expression_type(value, scope, functions)?;
            }
            Ok(ValueType::Unknown)
        }
    }
}

fn block_guarantees_termination(block: &BlockStatement) -> bool {
    let Some(last) = block.statements.last() else {
        return false;
    };
    statement_guarantees_termination(last)
}

fn statement_guarantees_termination(stmt: &Statement) -> bool {
    match stmt {
        Statement::ReturnStatement { .. } | Statement::ReturnError { .. } => true,
        Statement::IfStatement {
            then_block,
            else_block,
            ..
        } => {
            let Some(else_block) = else_block else {
                return false;
            };
            block_guarantees_termination(then_block) && block_guarantees_termination(else_block)
        }
        Statement::BlockStatement { statements, .. } | Statement::OnErrorBlock { statements, .. } => {
            let Some(last) = statements.last() else {
                return false;
            };
            statement_guarantees_termination(last)
        }
        _ => false,
    }
}

fn contains_variable(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::VariableReference(v) => v == name,
        Expression::BinaryOp { left, right, .. } => {
            contains_variable(left, name)
                || right
                    .as_deref()
                    .map(|r| contains_variable(r, name))
                    .unwrap_or(false)
        }
        Expression::StructConstruction { fields } => {
            fields.values().any(|v| contains_variable(v, name))
        }
        Expression::Call { args, .. } => args.iter().any(|a| contains_variable(a, name)),
        _ => false,
    }
}
