use std::collections::{HashMap, HashSet};

use crate::ast_nodes::{BlockStatement, Expression, Program, Statement};

pub fn semantic_analyze(program: &Program) -> Result<(), String> {
    let mut functions = HashMap::new();
    let mut labels: HashMap<String, Vec<String>> = HashMap::new();
    for stmt in &program.statements {
        if let Statement::FunctionDef { name, is_danger, .. } = stmt {
            functions.insert(name.clone(), *is_danger);
        }
        if let Statement::LabelDecl { name, variants } = stmt {
            labels.insert(name.clone(), variants.clone());
        }
    }
    validate_error_code_label(&labels)?;
    let mut scope = HashSet::new();
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

fn analyze_statements(
    statements: &[Statement],
    scope: &mut HashSet<String>,
    functions: &HashMap<String, bool>,
    labels: &HashMap<String, Vec<String>>,
    current_fn_is_danger: Option<bool>,
) -> Result<(), String> {
    for stmt in statements {
        analyze_statement(stmt, scope, functions, labels, current_fn_is_danger)?;
    }
    Ok(())
}

fn analyze_statement(
    stmt: &Statement,
    scope: &mut HashSet<String>,
    functions: &HashMap<String, bool>,
    labels: &HashMap<String, Vec<String>>,
    current_fn_is_danger: Option<bool>,
) -> Result<(), String> {
    match stmt {
        Statement::VarDecl { name, value, .. } => {
            if scope.contains(name) {
                return Err(format!(
                    "Redeclaration in same scope: '{}' is already defined.",
                    name
                ));
            }
            if contains_variable(value, name) {
                return Err(format!(
                    "Invalid initialization: '{}' is used in its own initializing expression.",
                    name
                ));
            }
            analyze_expression(value, scope)?;
            scope.insert(name.clone());
            Ok(())
        }
        Statement::Assignment { target, value } => {
            if !scope.contains(target) {
                return Err(format!(
                    "Use-before-definition: '{}' is not defined in current scope.",
                    target
                ));
            }
            analyze_expression(value, scope)?;
            Ok(())
        }
        Statement::FunctionDef { name, params, body, .. } => {
            scope.insert(name.clone());
            let mut fn_scope = scope.clone();
            for p in params {
                fn_scope.insert(p.name.clone());
            }
            let is_danger = *functions.get(name).unwrap_or(&false);
            analyze_block(body, &mut fn_scope, functions, labels, Some(is_danger))?;
            if is_danger && !block_guarantees_termination(body) {
                return Err(format!(
                    "danger fn '{}' must end with explicit return/return error on all paths.",
                    name
                ));
            }
            Ok(())
        }
        Statement::IfStatement { condition, then_block, else_block } => {
            analyze_expression(condition, scope)?;
            let mut then_scope = scope.clone();
            analyze_block(then_block, &mut then_scope, functions, labels, current_fn_is_danger)?;
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                analyze_block(else_block, &mut else_scope, functions, labels, current_fn_is_danger)?;
            }
            Ok(())
        }
        Statement::ForLoop { initialization, condition, update, body } => {
            let mut loop_scope = scope.clone();

            if let Some(init) = initialization {
                if let Expression::VariableReference(name) = init.as_ref() {
                    loop_scope.insert(name.clone());
                } else {
                    analyze_expression(init, &loop_scope)?;
                }
            }
            if let Some(cond) = condition {
                analyze_expression(cond, &loop_scope)?;
            }
            if let Some(upd) = update {
                analyze_expression(upd, &loop_scope)?;
            }

            analyze_block(body, &mut loop_scope, functions, labels, current_fn_is_danger)
        }
        Statement::WhenBlock { when_expression, cases, else_block } => {
            analyze_expression(when_expression, scope)?;
            for (case_exprs, block) in cases {
                for expr in case_exprs {
                    analyze_expression(expr, scope)?;
                }
                let mut case_scope = scope.clone();
                analyze_block(block, &mut case_scope, functions, labels, current_fn_is_danger)?;
            }
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                analyze_block(else_block, &mut else_scope, functions, labels, current_fn_is_danger)?;
            }
            Ok(())
        }
        Statement::WhileLoop { condition, body } => {
            analyze_expression(condition, scope)?;
            let mut while_scope = scope.clone();
            analyze_block(body, &mut while_scope, functions, labels, current_fn_is_danger)
        }
        Statement::LoopStatement { body } => {
            let mut local_scope = scope.clone();
            analyze_block(body, &mut local_scope, functions, labels, current_fn_is_danger)
        }
        Statement::OnErrorBlock { statements: body_statements } => {
            let mut local_scope = scope.clone();
            analyze_statements(body_statements, &mut local_scope, functions, labels, current_fn_is_danger)
        }
        Statement::BlockStatement { statements } => {
            let mut local_scope = scope.clone();
            analyze_statements(statements, &mut local_scope, functions, labels, current_fn_is_danger)
        }
        Statement::OnBlock { trigger } => {
            if trigger == "error" {
                return Err(
                    "Unsupported context: 'on error { ... }' is not yet semantically bound to a danger call."
                        .to_string(),
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
            if !functions.get(call_name).copied().unwrap_or(false) {
                return Err(format!(
                    "on error requires danger fn call: '{}' is not declared as danger.",
                    call_name
                ));
            }
            if !scope.contains(target) {
                return Err(format!(
                    "Use-before-definition: '{}' is not defined in current scope.",
                    target
                ));
            }
            for arg in args {
                analyze_expression(arg, scope)?;
            }
            let mut on_error_scope = scope.clone();
            analyze_block(on_error, &mut on_error_scope, functions, labels, current_fn_is_danger)
        }
        Statement::DangerCallOnError { call_name, args, on_error, .. } => {
            if !functions.get(call_name).copied().unwrap_or(false) {
                return Err(format!(
                    "on error requires danger fn call: '{}' is not declared as danger.",
                    call_name
                ));
            }
            for arg in args {
                analyze_expression(arg, scope)?;
            }
            let mut on_error_scope = scope.clone();
            analyze_block(on_error, &mut on_error_scope, functions, labels, current_fn_is_danger)
        }
        Statement::ReturnError { code } => {
            if current_fn_is_danger != Some(true) {
                return Err("return error is allowed only inside danger fn.".to_string());
            }
            let Some(error_codes) = labels.get("ErrorCode") else {
                return Err("return error requires label ErrorCode declaration.".to_string());
            };
            if !error_codes.iter().any(|v| v == code) {
                return Err(format!("Unknown ErrorCode variant: '{}'.", code));
            }
            Ok(())
        }
        Statement::ReturnStatement { value } => {
            if let Some(expr) = value {
                analyze_expression(expr, scope)?;
            }
            Ok(())
        }
        Statement::LabelDecl { .. } | Statement::StructDecl { .. } => Ok(()),
    }
}

fn analyze_block(
    block: &Box<BlockStatement>,
    scope: &mut HashSet<String>,
    functions: &HashMap<String, bool>,
    labels: &HashMap<String, Vec<String>>,
    current_fn_is_danger: Option<bool>,
) -> Result<(), String> {
    analyze_statements(&block.statements, scope, functions, labels, current_fn_is_danger)
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
        Statement::BlockStatement { statements } | Statement::OnErrorBlock { statements } => {
            let Some(last) = statements.last() else {
                return false;
            };
            statement_guarantees_termination(last)
        }
        _ => false,
    }
}

fn analyze_expression(expr: &Expression, scope: &HashSet<String>) -> Result<(), String> {
    match expr {
        Expression::LiteralInt(_) | Expression::LiteralFloat(_) => Ok(()),
        Expression::VariableReference(name) => {
            if scope.contains(name) {
                Ok(())
            } else {
                Err(format!("Use-before-definition: '{}' is not defined in current scope.", name))
            }
        }
        Expression::BinaryOp { left, right, .. } => {
            analyze_expression(left, scope)?;
            if let Some(right) = right {
                analyze_expression(right, scope)?;
            }
            Ok(())
        }
        Expression::StructConstruction { fields } => {
            for value in fields.values() {
                analyze_expression(value, scope)?;
            }
            Ok(())
        }
    }
}

fn contains_variable(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::VariableReference(v) => v == name,
        Expression::BinaryOp { left, right, .. } => {
            contains_variable(left, name)
                || right.as_deref().map(|r| contains_variable(r, name)).unwrap_or(false)
        }
        Expression::StructConstruction { fields } => fields.values().any(|v| contains_variable(v, name)),
        _ => false,
    }
}
