use std::collections::HashSet;

use crate::ast_nodes::{BlockStatement, Expression, Program, Statement};

pub fn semantic_analyze(program: &Program) -> Result<(), String> {
    let mut scope = HashSet::new();
    analyze_statements(&program.statements, &mut scope)
}

fn analyze_statements(statements: &[Statement], scope: &mut HashSet<String>) -> Result<(), String> {
    for stmt in statements {
        analyze_statement(stmt, scope)?;
    }
    Ok(())
}

fn analyze_statement(stmt: &Statement, scope: &mut HashSet<String>) -> Result<(), String> {
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
            analyze_block(body, &mut fn_scope)
        }
        Statement::IfStatement { condition, then_block, else_block } => {
            analyze_expression(condition, scope)?;
            let mut then_scope = scope.clone();
            analyze_block(then_block, &mut then_scope)?;
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                analyze_block(else_block, &mut else_scope)?;
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

            analyze_block(body, &mut loop_scope)
        }
        Statement::WhenBlock { when_expression, cases, else_block } => {
            analyze_expression(when_expression, scope)?;
            for (case_exprs, block) in cases {
                for expr in case_exprs {
                    analyze_expression(expr, scope)?;
                }
                let mut case_scope = scope.clone();
                analyze_block(block, &mut case_scope)?;
            }
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                analyze_block(else_block, &mut else_scope)?;
            }
            Ok(())
        }
        Statement::WhileLoop { condition, body } => {
            analyze_expression(condition, scope)?;
            let mut while_scope = scope.clone();
            analyze_block(body, &mut while_scope)
        }
        Statement::LoopStatement { body } => {
            let mut local_scope = scope.clone();
            analyze_block(body, &mut local_scope)
        }
        Statement::OnErrorBlock { statements: body_statements } => {
            let mut local_scope = scope.clone();
            analyze_statements(body_statements, &mut local_scope)
        }
        Statement::BlockStatement { statements } => {
            let mut local_scope = scope.clone();
            analyze_statements(statements, &mut local_scope)
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
            args,
            on_error,
            ..
        } => {
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
            analyze_block(on_error, &mut on_error_scope)
        }
        Statement::DangerCallOnError { args, on_error, .. } => {
            for arg in args {
                analyze_expression(arg, scope)?;
            }
            let mut on_error_scope = scope.clone();
            analyze_block(on_error, &mut on_error_scope)
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

fn analyze_block(block: &Box<BlockStatement>, scope: &mut HashSet<String>) -> Result<(), String> {
    analyze_statements(&block.statements, scope)
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
