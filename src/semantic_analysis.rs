use std::collections::HashMap;

use crate::ast_nodes::{BlockStatement, Expression, ForLoopStyle, FunctionParam, Program, Statement};
use crate::builtins::{builtin_arity, builtin_from_name, Builtin};
use crate::diagnostics::{format_diagnostic, DiagnosticKind};

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueType {
    Int,
    Float,
    Bool,
    Char,
    Text,
    List(Box<ValueType>),
    Unknown,
}

#[derive(Clone, Debug)]
struct FunctionSig {
    is_danger: bool,
    return_type: Option<ValueType>,
    param_types: Vec<ValueType>,
}

#[derive(Clone)]
struct FnContext {
    is_danger: bool,
    return_type: Option<ValueType>,
}

const SEM_REDECLARATION: &str = "SC-SEM-010";
const SEM_INVALID_INIT: &str = "SC-SEM-011";
const SEM_USE_BEFORE_DEF: &str = "SC-SEM-012";
const SEM_TYPE_MISMATCH: &str = "SC-SEM-020";
const SEM_UNKNOWN_FUNCTION: &str = "SC-SEM-030";
const SEM_ARG_COUNT: &str = "SC-SEM-031";
const SEM_ARG_TYPE: &str = "SC-SEM-032";
const SEM_BUILTIN_ARG: &str = "SC-SEM-033";
const SEM_INVALID_CONTEXT: &str = "SC-SEM-040";
const SEM_RETURN_RULE: &str = "SC-SEM-050";
const SEM_ERRORCODE_RULE: &str = "SC-SEM-051";
const SEM_INTERNAL: &str = "SC-SEM-900";

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
        | Statement::ListPush { loc, .. }
        | Statement::ListPopOnError { loc, .. }
        | Statement::ReturnError { loc, .. }
        | Statement::ReturnStatement { loc, .. }
        | Statement::ExpressionStatement { loc, .. }
        | Statement::BlockStatement { loc, .. }
        | Statement::OnErrorBlock { loc, .. } => Some((loc.line, loc.column)),
    }
}

fn sem_err(code: &'static str, msg: String) -> String {
    format_diagnostic(DiagnosticKind::Semantic, Some(code), msg, None, None, None)
}

fn err_at_code(stmt: &Statement, code: &'static str, msg: String) -> String {
    if let Some((line, col)) = statement_loc(stmt) {
        format_diagnostic(
            DiagnosticKind::Semantic,
            Some(code),
            msg,
            Some(line),
            Some(col),
            None,
        )
    } else {
        sem_err(code, msg)
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

pub fn semantic_style_warnings(program: &Program) -> Vec<String> {
    fn is_known_type_name(type_name: &str) -> bool {
        matches!(
            type_name,
            "Int"
                | "Float"
                | "Text"
                | "Path"
                | "List"
                | "Vec2"
                | "Vec3"
                | "Vec4"
                | "Bool"
                | "Char"
                | "bool"
                | "char"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "f32"
                | "f64"
        )
    }

    fn warn_type_style(type_name: &str, line: u32, col: u32, out: &mut Vec<String>) {
        if !is_known_type_name(type_name) {
            let msg = format!(
                "style warning at line {}, col {}: non-canonical type spelling '{}'.",
                line, col, type_name
            );
            out.push(msg);
            return;
        }
        if type_name == "bool" {
            out.push(format!(
                "style warning at line {}, col {}: prefer 'Bool' over 'bool' in showcase-style code.",
                line, col
            ));
        } else if type_name == "char" {
            out.push(format!(
                "style warning at line {}, col {}: prefer 'Char' over 'char' in showcase-style code.",
                line, col
            ));
        }
    }

    fn visit_statements(stmts: &[Statement], out: &mut Vec<String>) {
        for stmt in stmts {
            match stmt {
                Statement::VarDecl {
                    declared_type,
                    loc,
                    ..
                } => {
                    if let Some(dt) = declared_type {
                        if let Some(elem) = dt.strip_suffix(" List") {
                            warn_type_style(elem.trim(), loc.line, loc.column, out);
                        } else {
                            warn_type_style(dt, loc.line, loc.column, out);
                        }
                    }
                }
                Statement::FunctionDef {
                    params,
                    returns,
                    body,
                    loc,
                    ..
                } => {
                    for p in params {
                        if let Some(pt) = p.param_type.as_deref() {
                            warn_type_style(pt, loc.line, loc.column, out);
                        }
                    }
                    if let Some(rt) = returns.as_deref() {
                        if let Some(elem) = rt.strip_suffix(" List") {
                            warn_type_style(elem.trim(), loc.line, loc.column, out);
                        } else {
                            warn_type_style(rt, loc.line, loc.column, out);
                        }
                    }
                    visit_statements(&body.statements, out);
                }
                Statement::IfStatement {
                    then_block,
                    else_block,
                    ..
                } => {
                    visit_statements(&then_block.statements, out);
                    if let Some(b) = else_block {
                        visit_statements(&b.statements, out);
                    }
                }
                Statement::ForLoop { style, body, loc, .. } => {
                    if *style == ForLoopStyle::ForIn {
                        out.push(format!(
                            "style warning at line {}, col {}: prefer 'iterate <collection> as <item>' over 'for <item> in <collection>' in showcase-style code.",
                            loc.line, loc.column
                        ));
                    }
                    visit_statements(&body.statements, out);
                }
                Statement::WhileLoop { body, .. }
                | Statement::LoopStatement { body, .. } => {
                    visit_statements(&body.statements, out);
                }
                Statement::WhenBlock {
                    cases,
                    else_block,
                    ..
                } => {
                    for (_, b) in cases {
                        visit_statements(&b.statements, out);
                    }
                    if let Some(b) = else_block {
                        visit_statements(&b.statements, out);
                    }
                }
                Statement::OnErrorBlock { statements, .. }
                | Statement::BlockStatement { statements, .. } => {
                    visit_statements(statements, out);
                }
                Statement::DangerAssignOnError { on_error, .. }
                | Statement::DangerCallOnError { on_error, .. }
                | Statement::ListPopOnError { on_error, .. } => {
                    visit_statements(&on_error.statements, out);
                }
                _ => {}
            }
        }
    }

    let mut warnings = Vec::new();
    visit_statements(&program.statements, &mut warnings);
    warnings
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

fn parse_primitive_type_name(name: &str) -> ValueType {
    match name {
        "Int" | "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" => ValueType::Int,
        "Float" | "f64" | "f32" => ValueType::Float,
        "bool" | "Bool" => ValueType::Bool,
        "char" | "Char" => ValueType::Char,
        "Text" | "Path" => ValueType::Text,
        _ => ValueType::Unknown,
    }
}

fn parse_type_name(name: &str) -> ValueType {
    if let Some(elem) = name.strip_suffix(" List") {
        return ValueType::List(Box::new(parse_primitive_type_name(elem.trim())));
    }
    parse_primitive_type_name(name)
}

fn can_assign(target: &ValueType, source: &ValueType) -> bool {
    if target == source {
        return true;
    }
    match (target, source) {
        (ValueType::Float, ValueType::Int) => true,
        (ValueType::List(t), ValueType::List(s)) => {
            **s == ValueType::Unknown || can_assign(t, s)
        }
        _ => false,
    }
}

fn validate_call_args(
    name: &str,
    args: &[Expression],
    sig: &FunctionSig,
    scope: &HashMap<String, ValueType>,
    functions: &HashMap<String, FunctionSig>,
) -> Result<(), String> {
    if args.len() != sig.param_types.len() {
        return Err(sem_err(
            SEM_ARG_COUNT,
            format!(
            "argument count mismatch for '{}': expected {}, got {}.",
            name,
            sig.param_types.len(),
            args.len()
        )));
    }
    for (arg, expected_ty) in args.iter().zip(sig.param_types.iter().cloned()) {
        let actual_ty = infer_expression_type(arg, scope, functions)?;
        if !can_assign(&expected_ty, &actual_ty) {
            return Err(sem_err(
                SEM_ARG_TYPE,
                format!(
                "argument type mismatch for '{}': expected {:?}, got {:?}.",
                name, expected_ty, actual_ty
            )));
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
        analyze_statement(stmt, scope, functions, labels, fn_ctx.clone())?;
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
                return Err(err_at_code(
                    stmt,
                    SEM_REDECLARATION,
                    format!("redeclaration in same scope: '{}' is already defined.", name),
                ));
            }
            if contains_variable(value, name) {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_INIT,
                    format!("invalid initialization: '{}' is used in its own initializing expression.", name),
                ));
            }
            let value_ty = infer_expression_type(value, scope, functions)?;
            let final_ty = if let Some(tn) = declared_type {
                let declared = parse_type_name(tn);
                if !can_assign(&declared, &value_ty) {
                    return Err(format!(
                        "{}",
                        err_at_code(
                            stmt,
                            SEM_TYPE_MISMATCH,
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
            let Some(target_ty) = scope.get(target).cloned() else {
                return Err(format!(
                    "{}",
                    err_at_code(
                        stmt,
                        SEM_USE_BEFORE_DEF,
                        format!("use-before-definition: '{}' is not defined in current scope.", target),
                    )
                ));
            };
            let value_ty = infer_expression_type(value, scope, functions)?;
            if !can_assign(&target_ty, &value_ty) {
                return Err(format!(
                    "{}",
                    err_at_code(
                        stmt,
                        SEM_TYPE_MISMATCH,
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
                return Err(sem_err(
                    SEM_INTERNAL,
                    format!("internal error: missing function signature for '{}'.", name),
                ));
            };
            let local_ctx = FnContext {
                is_danger: sig.is_danger,
                return_type: sig.return_type.clone(),
            };
            analyze_block(body, &mut fn_scope, functions, labels, Some(local_ctx))?;
            if sig.is_danger && !block_guarantees_termination(body) {
                return Err(format!(
                    "{}",
                    err_at_code(
                        stmt,
                        SEM_RETURN_RULE,
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
                return Err(err_at_code(stmt, SEM_TYPE_MISMATCH, "if condition must be bool.".to_string()));
            }
            let mut then_scope = scope.clone();
            analyze_block(then_block, &mut then_scope, functions, labels, fn_ctx.clone())?;
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                analyze_block(else_block, &mut else_scope, functions, labels, fn_ctx.clone())?;
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
                    let inferred_item_ty = if let Some(coll) = condition {
                        match infer_expression_type(coll, &loop_scope, functions)? {
                            ValueType::List(elem_ty) => (*elem_ty).clone(),
                            ValueType::Text => ValueType::Char,
                            other => {
                                return Err(err_at_code(
                                    stmt,
                                    SEM_TYPE_MISMATCH,
                                    format!(
                                        "for/iterate expects List or Text collection, got {:?}.",
                                        other
                                    ),
                                ))
                            }
                        }
                    } else {
                        ValueType::Unknown
                    };
                    loop_scope.insert(name.clone(), inferred_item_ty);
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
                    if !can_assign(&when_ty, &case_ty) && !can_assign(&case_ty, &when_ty) {
                        return Err(err_at_code(
                            stmt,
                            SEM_TYPE_MISMATCH,
                            format!(
                                "type mismatch in when-case: case type {:?} incompatible with when type {:?}.",
                                case_ty, when_ty
                            ),
                        ));
                    }
                }
                let mut case_scope = scope.clone();
                analyze_block(block, &mut case_scope, functions, labels, fn_ctx.clone())?;
            }
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                analyze_block(else_block, &mut else_scope, functions, labels, fn_ctx.clone())?;
            }
            Ok(())
        }
        Statement::WhileLoop { condition, body, .. } => {
            let cty = infer_expression_type(condition, scope, functions)?;
            if cty != ValueType::Bool {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    "while condition must be bool.".to_string(),
                ));
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
                    err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
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
                return Err(err_at_code(
                    stmt,
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in on error call.", call_name),
                ));
            };
            if !sig.is_danger {
                return Err(format!(
                    "{}",
                    err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
                        format!("on error requires danger fn call: '{}' is not declared as danger.", call_name),
                    )
                ));
            }
            if !scope.contains_key(target) {
                return Err(format!(
                    "{}",
                    err_at_code(
                        stmt,
                        SEM_USE_BEFORE_DEF,
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
                return Err(err_at_code(
                    stmt,
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in on error call.", call_name),
                ));
            };
            if !sig.is_danger {
                return Err(format!(
                    "{}",
                    err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
                        format!("on error requires danger fn call: '{}' is not declared as danger.", call_name),
                    )
                ));
            }
            validate_call_args(call_name, args, sig, scope, functions)?;
            let mut on_error_scope = scope.clone();
            analyze_block(on_error, &mut on_error_scope, functions, labels, fn_ctx)
        }
        Statement::ListPush { list_name, value, .. } => {
            let Some(list_ty) = scope.get(list_name).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!("use-before-definition: '{}' is not defined in current scope.", list_name),
                ));
            };
            let value_ty = infer_expression_type(value, scope, functions)?;
            match list_ty {
                ValueType::List(elem_ty) => {
                    if !can_assign(&elem_ty, &value_ty) {
                        return Err(err_at_code(
                            stmt,
                            SEM_TYPE_MISMATCH,
                            format!(
                                "type mismatch in list push '{}': cannot push {:?} into {:?}.",
                                list_name, value_ty, elem_ty
                            ),
                        ));
                    }
                    Ok(())
                }
                other => Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    format!("push is supported only for List, got {:?}.", other),
                )),
            }
        }
        Statement::ListPopOnError {
            target,
            list_name,
            on_error,
            ..
        } => {
            let Some(target_ty) = scope.get(target).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!("use-before-definition: '{}' is not defined in current scope.", target),
                ));
            };
            let Some(list_ty) = scope.get(list_name).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!("use-before-definition: '{}' is not defined in current scope.", list_name),
                ));
            };
            let elem_ty = match list_ty {
                ValueType::List(elem_ty) => (*elem_ty).clone(),
                other => {
                    return Err(err_at_code(
                        stmt,
                        SEM_TYPE_MISMATCH,
                        format!("pop is supported only for List, got {:?}.", other),
                    ))
                }
            };
            if !can_assign(&target_ty, &elem_ty) {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    format!(
                        "type mismatch in list pop '{}': cannot assign {:?} to {:?}.",
                        list_name, elem_ty, target_ty
                    ),
                ));
            }
            let mut on_error_scope = scope.clone();
            analyze_block(on_error, &mut on_error_scope, functions, labels, fn_ctx)
        }
        Statement::ReturnError { code, .. } => {
            if fn_ctx.map(|c| c.is_danger) != Some(true) {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    "return error is allowed only inside danger fn.".to_string(),
                ));
            }
            let Some(error_codes) = labels.get("ErrorCode") else {
                return Err(err_at_code(
                    stmt,
                    SEM_ERRORCODE_RULE,
                    "return error requires label ErrorCode declaration.".to_string(),
                ));
            };
            if !error_codes.iter().any(|v| v == code) {
                return Err(err_at_code(
                    stmt,
                    SEM_ERRORCODE_RULE,
                    format!("unknown ErrorCode variant: '{}'.", code),
                ));
            }
            Ok(())
        }
        Statement::ReturnStatement { value, .. } => {
            if let Some(ctx) = fn_ctx {
                if let Some(expr) = value {
                    let actual = infer_expression_type(expr, scope, functions)?;
                    if let Some(expected) = ctx.return_type.as_ref() {
                        if !can_assign(expected, &actual) {
                            return Err(err_at_code(
                                stmt,
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "type mismatch in return: cannot return {:?} where {:?} expected.",
                                    actual, expected
                                ),
                            ));
                        }
                    }
                } else if !ctx.is_danger && ctx.return_type.is_some() {
                    return Err(err_at_code(
                        stmt,
                        SEM_RETURN_RULE,
                        "non-danger function with return type must return a value.".to_string(),
                    ));
                }
            } else if let Some(expr) = value {
                let _ = infer_expression_type(expr, scope, functions)?;
            }
            Ok(())
        }
        Statement::ExpressionStatement { expr, .. } => {
            let _ = infer_expression_type(expr, scope, functions)?;
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
        Expression::LiteralString(_) => Ok(ValueType::Text),
        Expression::ListLiteral(items) => {
            if items.is_empty() {
                return Ok(ValueType::List(Box::new(ValueType::Unknown)));
            }
            let mut elem_ty = infer_expression_type(&items[0], scope, functions)?;
            for item in &items[1..] {
                let item_ty = infer_expression_type(item, scope, functions)?;
                if can_assign(&elem_ty, &item_ty) {
                    continue;
                }
                if can_assign(&item_ty, &elem_ty) {
                    elem_ty = item_ty;
                    continue;
                }
                return Err(sem_err(
                    SEM_TYPE_MISMATCH,
                    format!(
                        "type mismatch in list literal: cannot mix {:?} and {:?}.",
                        elem_ty, item_ty
                    ),
                ));
            }
            Ok(ValueType::List(Box::new(elem_ty)))
        }
        Expression::VariableReference(name) => scope
            .get(name)
            .cloned()
            .ok_or_else(|| sem_err(SEM_USE_BEFORE_DEF, format!("use-before-definition: '{}' is not defined in current scope.", name))),
        Expression::Index { base, index } => {
            let base_ty = infer_expression_type(base, scope, functions)?;
            let idx_ty = infer_expression_type(index, scope, functions)?;
            if idx_ty != ValueType::Int {
                return Err(sem_err(
                    SEM_TYPE_MISMATCH,
                    format!("index access requires Int index, got {:?}.", idx_ty),
                ));
            }
            match base_ty {
                ValueType::List(elem_ty) => Ok((*elem_ty).clone()),
                ValueType::Text => Ok(ValueType::Char),
                other => Err(sem_err(
                    SEM_TYPE_MISMATCH,
                    format!("index access is supported only for List/Text, got {:?}.", other),
                )),
            }
        }
        Expression::Call { name, args } => {
            if let Some(builtin) = builtin_from_name(name) {
                let expected_arity = builtin_arity(builtin);
                if args.len() != expected_arity {
                    return Err(sem_err(
                        SEM_BUILTIN_ARG,
                        format!(
                            "builtin '{}' expects {} arguments, got {}.",
                            name,
                            expected_arity,
                            args.len()
                        ),
                    ));
                }
                return match builtin {
                    Builtin::Len => {
                        let arg_ty = infer_expression_type(&args[0], scope, functions)?;
                        match arg_ty {
                            ValueType::List(_) | ValueType::Text => Ok(ValueType::Int),
                            _ => Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'len' expects List or Text, got {:?}.", arg_ty),
                            )),
                        }
                    }
                    Builtin::Contains => {
                        let hay_ty = infer_expression_type(&args[0], scope, functions)?;
                        let needle_ty = infer_expression_type(&args[1], scope, functions)?;
                        if hay_ty != ValueType::Text || needle_ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin 'contains' expects (Text, Text), got ({:?}, {:?}).",
                                    hay_ty, needle_ty
                                ),
                            ));
                        }
                        Ok(ValueType::Bool)
                    }
                    Builtin::Find => {
                        let hay_ty = infer_expression_type(&args[0], scope, functions)?;
                        let needle_ty = infer_expression_type(&args[1], scope, functions)?;
                        if hay_ty != ValueType::Text || needle_ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin 'find' expects (Text, Text), got ({:?}, {:?}).",
                                    hay_ty, needle_ty
                                ),
                            ));
                        }
                        Ok(ValueType::Int)
                    }
                    Builtin::Slice => {
                        let text_ty = infer_expression_type(&args[0], scope, functions)?;
                        let start_ty = infer_expression_type(&args[1], scope, functions)?;
                        let end_ty = infer_expression_type(&args[2], scope, functions)?;
                        if text_ty != ValueType::Text || start_ty != ValueType::Int || end_ty != ValueType::Int {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin 'slice' expects (Text, Int, Int), got ({:?}, {:?}, {:?}).",
                                    text_ty, start_ty, end_ty
                                ),
                            ));
                        }
                        Ok(ValueType::Text)
                    }
                    Builtin::Concat => {
                        let a = infer_expression_type(&args[0], scope, functions)?;
                        let b = infer_expression_type(&args[1], scope, functions)?;
                        if a != ValueType::Text || b != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'concat' expects (Text, Text), got ({:?}, {:?}).", a, b),
                            ));
                        }
                        Ok(ValueType::Text)
                    }
                    Builtin::FsList => {
                        let path_ty = infer_expression_type(&args[0], scope, functions)?;
                        if path_ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'fs.list' expects (Text), got ({:?}).", path_ty),
                            ));
                        }
                        Ok(ValueType::List(Box::new(ValueType::Text)))
                    }
                    Builtin::FsIsDir => {
                        let path_ty = infer_expression_type(&args[0], scope, functions)?;
                        if path_ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'fs.is_dir' expects (Text), got ({:?}).", path_ty),
                            ));
                        }
                        Ok(ValueType::Bool)
                    }
                    Builtin::FsJoin => {
                        let a = infer_expression_type(&args[0], scope, functions)?;
                        let b = infer_expression_type(&args[1], scope, functions)?;
                        if a != ValueType::Text || b != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'fs.join' expects (Text, Text), got ({:?}, {:?}).", a, b),
                            ));
                        }
                        Ok(ValueType::Text)
                    }
                    Builtin::Args => Ok(ValueType::List(Box::new(ValueType::Text))),
                    Builtin::Output => {
                        let ty = infer_expression_type(&args[0], scope, functions)?;
                        match ty {
                            ValueType::Int | ValueType::Float | ValueType::Bool | ValueType::Char | ValueType::Text => Ok(ValueType::Int),
                            _ => Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'output' unsupported argument type: {:?}.", ty),
                            )),
                        }
                    }
                    Builtin::Input => {
                        let ty = infer_expression_type(&args[0], scope, functions)?;
                        if ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'input' expects (Text), got ({:?}).", ty),
                            ));
                        }
                        Ok(ValueType::Text)
                    }
                    Builtin::Read => {
                        let ty = infer_expression_type(&args[0], scope, functions)?;
                        if ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'read' expects (Text), got ({:?}).", ty),
                            ));
                        }
                        Ok(ValueType::Text)
                    }
                    Builtin::Write => {
                        let p = infer_expression_type(&args[0], scope, functions)?;
                        let d = infer_expression_type(&args[1], scope, functions)?;
                        if p != ValueType::Text || d != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'write' expects (Text, Text), got ({:?}, {:?}).", p, d),
                            ));
                        }
                        Ok(ValueType::Int)
                    }
                };
            }
            let Some(sig) = functions.get(name) else {
                return Err(sem_err(
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in expression call.", name),
                ));
            };
            validate_call_args(name, args, sig, scope, functions)?;
            Ok(sig.return_type.clone().unwrap_or(ValueType::Unknown))
        }
        Expression::BinaryOp { op, left, right } => {
            if op == "neg" {
                let lt = infer_expression_type(left, scope, functions)?;
                if lt == ValueType::Int || lt == ValueType::Float {
                    return Ok(lt);
                }
                return Err(sem_err(
                    SEM_TYPE_MISMATCH,
                    "unary '-' requires numeric operand.".to_string(),
                ));
            }
            if op == "not" {
                let lt = infer_expression_type(left, scope, functions)?;
                if lt == ValueType::Bool || lt == ValueType::Int {
                    return Ok(ValueType::Bool);
                }
                return Err(sem_err(
                    SEM_TYPE_MISMATCH,
                    "unary 'not' requires bool/int operand.".to_string(),
                ));
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
                        Err(sem_err(
                            SEM_TYPE_MISMATCH,
                            format!("operator '{}' requires numeric operands.", op),
                        ))
                    }
                }
                "==" | "!=" | "<" | ">" | "<=" | ">=" => Ok(ValueType::Bool),
                "and" | "or" | "xor" => {
                    if (lt == ValueType::Bool || lt == ValueType::Int)
                        && (rt == ValueType::Bool || rt == ValueType::Int)
                    {
                        Ok(ValueType::Bool)
                    } else {
                        Err(sem_err(
                            SEM_TYPE_MISMATCH,
                            format!("operator '{}' requires bool/int operands.", op),
                        ))
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
        Expression::Index { base, index } => {
            contains_variable(base, name) || contains_variable(index, name)
        }
        Expression::ListLiteral(items) => items.iter().any(|a| contains_variable(a, name)),
        Expression::LiteralString(_) => false,
        _ => false,
    }
}
