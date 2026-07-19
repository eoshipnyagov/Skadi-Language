use std::collections::{HashMap, HashSet};

use crate::ast_nodes::{
    BlockStatement, Expression, ForLoopStyle, FunctionParam, Program, Statement,
};
use crate::builtins::{Builtin, builtin_arity, builtin_from_name};
use crate::diagnostics::{DiagnosticKind, format_diagnostic};

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueType {
    Int,
    Float,
    Bool,
    Char,
    Text,
    Memory,
    Task(Option<Box<ValueType>>),
    Channel(Box<ValueType>),
    List(Box<ValueType>),
    Struct(String),
    Unknown,
}

#[derive(Clone, Debug)]
struct FunctionSig {
    is_danger: bool,
    return_type: Option<ValueType>,
    has_explicit_return: bool,
    param_types: Vec<ValueType>,
}

#[derive(Clone)]
struct FnContext {
    is_danger: bool,
    return_type: Option<ValueType>,
    self_struct: Option<String>,
    is_task_context: bool,
}

#[derive(Clone, Debug, Default)]
struct MemoryBinding {
    is_external: bool,
    is_cleared: bool,
}

#[derive(Clone, Debug)]
struct TaskBinding {
    result_type: Option<ValueType>,
    waited: bool,
    stopped: bool,
}

#[derive(Clone, Debug, Default)]
struct MemoryState {
    active_memory: Option<String>,
    memories: HashMap<String, MemoryBinding>,
    variable_memory: HashMap<String, String>,
    tasks: HashMap<String, TaskBinding>,
    channels: HashMap<String, ValueType>,
}

#[derive(Clone, Debug)]
struct StructInfo {
    fields: HashMap<String, ValueType>,
    methods: HashMap<String, FunctionSig>,
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
const SEM_MEMORY_RULE: &str = "SC-SEM-060";
const SEM_MEMORY_LIFETIME: &str = "SC-SEM-061";
const SEM_MEMORY_CAPABILITY: &str = "SC-SEM-062";
const SEM_TASK_RULE: &str = "SC-SEM-070";
const SEM_TASK_CAPABILITY: &str = "SC-SEM-071";
const SEM_CHANNEL_RULE: &str = "SC-SEM-080";
const SEM_INTERNAL: &str = "SC-SEM-900";

fn statement_loc(stmt: &Statement) -> Option<(u32, u32)> {
    match stmt {
        Statement::VarDecl { loc, .. }
        | Statement::MemoryDecl { loc, .. }
        | Statement::Assignment { loc, .. }
        | Statement::IncDec { loc, .. }
        | Statement::FieldAssignment { loc, .. }
        | Statement::FunctionDef { loc, .. }
        | Statement::IfStatement { loc, .. }
        | Statement::ForLoop { loc, .. }
        | Statement::WhenBlock { loc, .. }
        | Statement::WhileLoop { loc, .. }
        | Statement::LoopStatement { loc, .. }
        | Statement::BreakStatement { loc }
        | Statement::ContinueStatement { loc }
        | Statement::PassStatement { loc }
        | Statement::LabelDecl { loc, .. }
        | Statement::StructDecl { loc, .. }
        | Statement::OnBlock { loc, .. }
        | Statement::DangerAssignOnError { loc, .. }
        | Statement::DangerCallOnError { loc, .. }
        | Statement::ListPush { loc, .. }
        | Statement::ListPopOnError { loc, .. }
        | Statement::PlaceIn { loc, .. }
        | Statement::MemoryClear { loc, .. }
        | Statement::StopTask { loc, .. }
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
    let mut structs: HashMap<String, StructInfo> = HashMap::new();
    let task_context_functions = collect_task_context_functions(&program.statements);

    for stmt in &program.statements {
        if let Statement::FunctionDef {
            name,
            is_danger,
            returns,
            params,
            ..
        } = stmt
        {
            if let Some(return_name) = returns.as_deref() {
                let return_ty = parse_type_name(return_name);
                ensure_memory_type_allowed(stmt, &return_ty, "function return type", false)?;
                ensure_task_type_allowed(stmt, &return_ty, "function return type", false)?;
                ensure_channel_type_allowed(stmt, &return_ty, "function return type", false)?;
            }
            for param in params {
                if let Some(param_name) = param.param_type.as_deref() {
                    let param_ty = parse_type_name(param_name);
                    ensure_memory_type_allowed(
                        stmt,
                        &param_ty,
                        "function parameter type",
                        param_ty == ValueType::Memory,
                    )?;
                    ensure_task_type_allowed(stmt, &param_ty, "function parameter type", false)?;
                    ensure_channel_type_allowed(
                        stmt,
                        &param_ty,
                        "function parameter type",
                        matches!(param_ty, ValueType::Channel(_)),
                    )?;
                }
            }
            functions.insert(
                name.clone(),
                FunctionSig {
                    is_danger: *is_danger,
                    return_type: returns
                        .as_deref()
                        .map(parse_type_name)
                        .or(Some(ValueType::Int)),
                    has_explicit_return: returns.is_some(),
                    param_types: params.iter().map(param_type_or_default).collect(),
                },
            );
        }
        if let Statement::LabelDecl { name, variants, .. } = stmt {
            labels.insert(name.clone(), variants.clone());
        }
        if let Statement::StructDecl {
            name,
            fields,
            methods,
            ..
        } = stmt
        {
            let mut fmap = HashMap::new();
            for f in fields {
                let field_ty = parse_type_name(&f.field_type);
                ensure_memory_type_allowed(stmt, &field_ty, "struct field type", false)?;
                ensure_task_type_allowed(stmt, &field_ty, "struct field type", false)?;
                ensure_channel_type_allowed(stmt, &field_ty, "struct field type", false)?;
                fmap.insert(f.name.clone(), field_ty);
            }
            let mut mmap = HashMap::new();
            for m in methods {
                if let Some(return_name) = m.returns.as_deref() {
                    let return_ty = parse_type_name(return_name);
                    ensure_memory_type_allowed(
                        stmt,
                        &return_ty,
                        "struct method return type",
                        false,
                    )?;
                    ensure_task_type_allowed(stmt, &return_ty, "struct method return type", false)?;
                    ensure_channel_type_allowed(
                        stmt,
                        &return_ty,
                        "struct method return type",
                        false,
                    )?;
                }
                for param in &m.params {
                    if let Some(param_name) = param.param_type.as_deref() {
                        let param_ty = parse_type_name(param_name);
                        ensure_memory_type_allowed(
                            stmt,
                            &param_ty,
                            "struct method parameter type",
                            param_ty == ValueType::Memory,
                        )?;
                        ensure_task_type_allowed(
                            stmt,
                            &param_ty,
                            "struct method parameter type",
                            false,
                        )?;
                        ensure_channel_type_allowed(
                            stmt,
                            &param_ty,
                            "struct method parameter type",
                            matches!(param_ty, ValueType::Channel(_)),
                        )?;
                    }
                }
                mmap.insert(
                    m.name.clone(),
                    FunctionSig {
                        is_danger: m.is_danger,
                        return_type: m
                            .returns
                            .as_deref()
                            .map(parse_type_name)
                            .or(Some(ValueType::Int)),
                        has_explicit_return: m.returns.is_some(),
                        param_types: m.params.iter().map(param_type_or_default).collect(),
                    },
                );
            }
            structs.insert(
                name.clone(),
                StructInfo {
                    fields: fmap,
                    methods: mmap,
                },
            );
        }
    }

    // User-defined struct types may be declared after functions. Resolve function
    // signatures again once the complete struct table is available.
    for stmt in &program.statements {
        if let Statement::FunctionDef {
            name,
            returns,
            params,
            ..
        } = stmt
            && let Some(sig) = functions.get_mut(name)
        {
            sig.return_type = returns
                .as_deref()
                .map(|name| parse_declared_type_name(name, &structs))
                .or(Some(ValueType::Int));
            sig.param_types = params
                .iter()
                .map(|param| {
                    param
                        .param_type
                        .as_deref()
                        .map(|name| parse_declared_type_name(name, &structs))
                        .unwrap_or(ValueType::Int)
                })
                .collect();
        }
    }

    validate_error_code_label(&labels)?;

    let mut scope: HashMap<String, ValueType> = HashMap::new();
    let mut memory_state = MemoryState::default();
    analyze_statements(
        &program.statements,
        &mut scope,
        &mut memory_state,
        &functions,
        &labels,
        &structs,
        &task_context_functions,
        None,
        false,
    )?;
    validate_task_lifecycle(program)
}

pub fn semantic_style_warnings(program: &Program) -> Vec<String> {
    let user_types: std::collections::HashSet<String> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::StructDecl { name, .. } | Statement::LabelDecl { name, .. } => {
                Some(name.clone())
            }
            _ => None,
        })
        .collect();

    fn is_known_type_name(type_name: &str, user_types: &std::collections::HashSet<String>) -> bool {
        if type_name == "Task" {
            return true;
        }
        if let Some(inner) = type_name
            .strip_prefix("Task(")
            .and_then(|s| s.strip_suffix(')'))
        {
            return is_known_type_name(inner.trim(), user_types);
        }
        if let Some(inner) = type_name
            .strip_prefix("Channel(")
            .and_then(|s| s.strip_suffix(')'))
        {
            return is_known_type_name(inner.trim(), user_types);
        }
        if user_types.contains(type_name) {
            return true;
        }
        matches!(
            type_name,
            "Int"
                | "Float"
                | "Memory"
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

    fn warn_type_style(
        type_name: &str,
        line: u32,
        col: u32,
        user_types: &std::collections::HashSet<String>,
        out: &mut Vec<String>,
    ) {
        if !is_known_type_name(type_name, user_types) {
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

    fn warn_memory_name(name: &str, line: u32, col: u32, out: &mut Vec<String>) {
        if !name.ends_with("_memory") {
            out.push(format!(
                "style warning at line {}, col {}: prefer '_memory' suffix for Memory handles like '{}_memory'.",
                line, col, name
            ));
        }
    }

    fn visit_expression_style(expr: &Expression, line: u32, col: u32, out: &mut Vec<String>) {
        match expr {
            Expression::ListLiteral(items) => {
                for item in items {
                    visit_expression_style(item, line, col, out);
                }
            }
            Expression::Index { base, index } => {
                visit_expression_style(base, line, col, out);
                visit_expression_style(index, line, col, out);
            }
            Expression::Call { args, .. } | Expression::RunTask { args, .. } => {
                for arg in args {
                    visit_expression_style(arg, line, col, out);
                }
            }
            Expression::WaitTask { .. } | Expression::Stopping => {}
            Expression::BinaryOp { left, right, .. } => {
                visit_expression_style(left, line, col, out);
                if let Some(right) = right {
                    visit_expression_style(right, line, col, out);
                }
            }
            Expression::StructConstruction { fields } => {
                for (field_name, field_value) in fields {
                    if let Expression::VariableReference(var_name) = field_value.as_ref()
                        && field_name == var_name
                    {
                        out.push(format!(
                            "style warning at line {line}, col {col}: avoid collapsed field init like '{{{field} = {field}}}' or '{{{field}}}'; prefer a distinct value name such as '{field}_value'.",
                            line = line,
                            col = col,
                            field = field_name
                        ));
                    }
                    visit_expression_style(field_value, line, col, out);
                }
            }
            _ => {}
        }
    }

    fn visit_statements(
        stmts: &[Statement],
        user_types: &std::collections::HashSet<String>,
        out: &mut Vec<String>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::VarDecl {
                    declared_type: Some(dt),
                    value,
                    loc,
                    ..
                } => {
                    if let Some(elem) = dt.strip_suffix(" List") {
                        warn_type_style(elem.trim(), loc.line, loc.column, user_types, out);
                    } else {
                        warn_type_style(dt, loc.line, loc.column, user_types, out);
                    }
                    visit_expression_style(value, loc.line, loc.column, out);
                }
                Statement::MemoryDecl {
                    name,
                    loc,
                    on_error,
                    ..
                } => {
                    warn_memory_name(name, loc.line, loc.column, out);
                    if let Some(on_error) = on_error {
                        visit_statements(&on_error.statements, user_types, out);
                    }
                }
                Statement::VarDecl { value, loc, .. } => {
                    visit_expression_style(value, loc.line, loc.column, out);
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
                            warn_type_style(pt, loc.line, loc.column, user_types, out);
                            if pt == "Memory" {
                                warn_memory_name(&p.name, loc.line, loc.column, out);
                            }
                        }
                    }
                    if let Some(rt) = returns.as_deref() {
                        if let Some(elem) = rt.strip_suffix(" List") {
                            warn_type_style(elem.trim(), loc.line, loc.column, user_types, out);
                        } else {
                            warn_type_style(rt, loc.line, loc.column, user_types, out);
                        }
                    }
                    visit_statements(&body.statements, user_types, out);
                }
                Statement::IfStatement {
                    condition,
                    then_block,
                    else_block,
                    loc,
                    ..
                } => {
                    visit_expression_style(condition, loc.line, loc.column, out);
                    visit_statements(&then_block.statements, user_types, out);
                    if let Some(b) = else_block {
                        visit_statements(&b.statements, user_types, out);
                    }
                }
                Statement::ForLoop {
                    initialization,
                    condition,
                    update,
                    style,
                    body,
                    loc,
                    ..
                } => {
                    if *style == ForLoopStyle::ForIn {
                        out.push(format!(
                            "style warning at line {}, col {}: prefer 'iterate <collection> as <item>' over 'for <item> in <collection>' in showcase-style code.",
                            loc.line, loc.column
                        ));
                    }
                    if let Some(initialization) = initialization {
                        visit_expression_style(initialization, loc.line, loc.column, out);
                    }
                    if let Some(condition) = condition {
                        visit_expression_style(condition, loc.line, loc.column, out);
                    }
                    if let Some(update) = update {
                        visit_expression_style(update, loc.line, loc.column, out);
                    }
                    visit_statements(&body.statements, user_types, out);
                }
                Statement::WhileLoop {
                    condition,
                    body,
                    loc,
                } => {
                    visit_expression_style(condition, loc.line, loc.column, out);
                    visit_statements(&body.statements, user_types, out);
                }
                Statement::LoopStatement { body, .. } => {
                    visit_statements(&body.statements, user_types, out);
                }
                Statement::PlaceIn { on_error, body, .. } => {
                    if let Some(on_error) = on_error {
                        visit_statements(&on_error.statements, user_types, out);
                    }
                    visit_statements(&body.statements, user_types, out);
                }
                Statement::WhenBlock {
                    when_expression,
                    cases,
                    else_block,
                    loc,
                    ..
                } => {
                    visit_expression_style(when_expression, loc.line, loc.column, out);
                    for (_, b) in cases {
                        visit_statements(&b.statements, user_types, out);
                    }
                    if let Some(b) = else_block {
                        visit_statements(&b.statements, user_types, out);
                    }
                }
                Statement::OnErrorBlock { statements, .. }
                | Statement::BlockStatement { statements, .. } => {
                    visit_statements(statements, user_types, out);
                }
                Statement::ListPopOnError { on_error, .. } => {
                    visit_statements(&on_error.statements, user_types, out);
                }
                Statement::Assignment { value, loc, .. }
                | Statement::FieldAssignment { value, loc, .. }
                | Statement::ListPush { value, loc, .. } => {
                    visit_expression_style(value, loc.line, loc.column, out);
                }
                Statement::DangerAssignOnError {
                    args,
                    on_error,
                    loc,
                    ..
                }
                | Statement::DangerCallOnError {
                    args,
                    on_error,
                    loc,
                    ..
                } => {
                    for arg in args {
                        visit_expression_style(arg, loc.line, loc.column, out);
                    }
                    visit_statements(&on_error.statements, user_types, out);
                }
                Statement::ReturnStatement {
                    value: Some(value),
                    loc,
                } => visit_expression_style(value, loc.line, loc.column, out),
                Statement::ExpressionStatement { expr, loc } => {
                    visit_expression_style(expr, loc.line, loc.column, out);
                }
                Statement::StructDecl { methods, .. } => {
                    for method in methods {
                        visit_statements(&method.body.statements, user_types, out);
                    }
                }
                _ => {}
            }
        }
    }

    let mut warnings = Vec::new();
    visit_statements(&program.statements, &user_types, &mut warnings);
    warnings
}

fn validate_error_code_label(labels: &HashMap<String, Vec<String>>) -> Result<(), String> {
    if let Some(error_codes) = labels.get("ErrorCode") {
        if error_codes.is_empty() {
            return Err(sem_err(
                SEM_ERRORCODE_RULE,
                "label ErrorCode must define at least one variant.".to_string(),
            ));
        }
        if error_codes[0] != "Ok" {
            return Err(sem_err(
                SEM_ERRORCODE_RULE,
                "label ErrorCode must start with 'Ok' variant.".to_string(),
            ));
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
        "Memory" => ValueType::Memory,
        "Text" | "Path" => ValueType::Text,
        _ => ValueType::Unknown,
    }
}

fn parse_type_name(name: &str) -> ValueType {
    if let Some(inner) = name.strip_prefix("Task(").and_then(|s| s.strip_suffix(')')) {
        return ValueType::Task(Some(Box::new(parse_type_name(inner.trim()))));
    }
    if name == "Task" {
        return ValueType::Task(None);
    }
    if let Some(inner) = name
        .strip_prefix("Channel(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return ValueType::Channel(Box::new(parse_type_name(inner.trim())));
    }
    if let Some(elem) = name.strip_suffix(" List") {
        return ValueType::List(Box::new(parse_type_name(elem.trim())));
    }
    parse_primitive_type_name(name)
}

fn collect_task_context_functions(statements: &[Statement]) -> HashSet<String> {
    fn visit_expr(expr: &Expression, out: &mut HashSet<String>) {
        match expr {
            Expression::RunTask { call_name, args } => {
                out.insert(call_name.clone());
                for arg in args {
                    visit_expr(arg, out);
                }
            }
            Expression::Call { args, .. } | Expression::ListLiteral(args) => {
                for arg in args {
                    visit_expr(arg, out);
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                visit_expr(left, out);
                if let Some(right) = right {
                    visit_expr(right, out);
                }
            }
            Expression::Index { base, index } => {
                visit_expr(base, out);
                visit_expr(index, out);
            }
            Expression::StructConstruction { fields } => {
                for value in fields.values() {
                    visit_expr(value, out);
                }
            }
            Expression::VariableReference(_)
            | Expression::MemberAccess { .. }
            | Expression::WaitTask { .. }
            | Expression::Stopping
            | Expression::LiteralInt(_)
            | Expression::LiteralFloat(_)
            | Expression::LiteralBool(_)
            | Expression::LiteralString(_) => {}
        }
    }

    fn visit_block(block: &BlockStatement, out: &mut HashSet<String>) {
        visit_statements(&block.statements, out);
    }

    fn visit_statements(statements: &[Statement], out: &mut HashSet<String>) {
        for stmt in statements {
            match stmt {
                Statement::VarDecl { value, .. }
                | Statement::Assignment { value, .. }
                | Statement::FieldAssignment { value, .. }
                | Statement::ListPush { value, .. } => visit_expr(value, out),
                Statement::ReturnStatement { value, .. } => {
                    if let Some(value) = value {
                        visit_expr(value, out);
                    }
                }
                Statement::ExpressionStatement { expr, .. } => visit_expr(expr, out),
                Statement::FunctionDef { body, .. } => visit_block(body, out),
                Statement::StructDecl { methods, .. } => {
                    for method in methods {
                        visit_block(&method.body, out);
                    }
                }
                Statement::IfStatement {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    visit_expr(condition, out);
                    visit_block(then_block, out);
                    if let Some(block) = else_block {
                        visit_block(block, out);
                    }
                }
                Statement::ForLoop {
                    initialization,
                    condition,
                    update,
                    body,
                    ..
                } => {
                    if let Some(expr) = initialization {
                        visit_expr(expr, out);
                    }
                    if let Some(expr) = condition {
                        visit_expr(expr, out);
                    }
                    if let Some(expr) = update {
                        visit_expr(expr, out);
                    }
                    visit_block(body, out);
                }
                Statement::WhenBlock {
                    when_expression,
                    cases,
                    else_block,
                    ..
                } => {
                    visit_expr(when_expression, out);
                    for (exprs, block) in cases {
                        for expr in exprs {
                            visit_expr(expr, out);
                        }
                        visit_block(block, out);
                    }
                    if let Some(block) = else_block {
                        visit_block(block, out);
                    }
                }
                Statement::WhileLoop {
                    condition, body, ..
                } => {
                    visit_expr(condition, out);
                    visit_block(body, out);
                }
                Statement::LoopStatement { body, .. }
                | Statement::OnBlock { body, .. }
                | Statement::PlaceIn { body, .. } => visit_block(body, out),
                Statement::DangerAssignOnError { args, on_error, .. }
                | Statement::DangerCallOnError { args, on_error, .. } => {
                    for arg in args {
                        visit_expr(arg, out);
                    }
                    visit_block(on_error, out);
                }
                Statement::ListPopOnError { on_error, .. } => visit_block(on_error, out),
                Statement::MemoryDecl { on_error, .. } => {
                    if let Some(block) = on_error {
                        visit_block(block, out);
                    }
                }
                Statement::BlockStatement { statements, .. }
                | Statement::OnErrorBlock { statements, .. } => visit_statements(statements, out),
                Statement::MemoryClear { .. }
                | Statement::StopTask { .. }
                | Statement::ReturnError { .. }
                | Statement::IncDec { .. }
                | Statement::BreakStatement { .. }
                | Statement::ContinueStatement { .. }
                | Statement::PassStatement { .. }
                | Statement::LabelDecl { .. } => {}
            }
        }
    }

    let mut out = HashSet::new();
    visit_statements(statements, &mut out);
    out
}

fn builtin_constant_type(name: &str) -> Option<ValueType> {
    match name {
        "PI" | "TAU" | "E" | "EPSILON" => Some(ValueType::Float),
        _ => None,
    }
}

fn is_numeric_type(ty: &ValueType) -> bool {
    matches!(ty, ValueType::Int | ValueType::Float)
}

fn ensure_numeric_args(name: &str, arg_tys: &[ValueType]) -> Result<(), String> {
    if arg_tys.iter().all(is_numeric_type) {
        Ok(())
    } else {
        Err(sem_err(
            SEM_TYPE_MISMATCH,
            format!(
                "builtin '{}' expects numeric arguments, got {:?}.",
                name, arg_tys
            ),
        ))
    }
}

fn numeric_result_type(arg_tys: &[ValueType], preserve_int: bool) -> ValueType {
    if preserve_int && arg_tys.iter().all(|ty| *ty == ValueType::Int) {
        ValueType::Int
    } else {
        ValueType::Float
    }
}

fn can_assign(target: &ValueType, source: &ValueType) -> bool {
    if *source == ValueType::Unknown || *target == ValueType::Unknown {
        return true;
    }
    if target == source {
        return true;
    }
    match (target, source) {
        (ValueType::Float, ValueType::Int) => true,
        (ValueType::List(t), ValueType::List(s)) => **s == ValueType::Unknown || can_assign(t, s),
        (ValueType::Task(None), ValueType::Task(None)) => true,
        (ValueType::Task(Some(t)), ValueType::Task(Some(s))) => can_assign(t, s),
        (ValueType::Channel(t), ValueType::Channel(s)) => {
            **s == ValueType::Unknown || can_assign(t, s)
        }
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_call_args(
    name: &str,
    args: &[Expression],
    sig: &FunctionSig,
    scope: &HashMap<String, ValueType>,
    memory_state: &MemoryState,
    functions: &HashMap<String, FunctionSig>,
    structs: &HashMap<String, StructInfo>,
    fn_ctx: Option<&FnContext>,
) -> Result<(), String> {
    if args.len() != sig.param_types.len() {
        return Err(sem_err(
            SEM_ARG_COUNT,
            format!(
                "argument count mismatch for '{}': expected {}, got {}.",
                name,
                sig.param_types.len(),
                args.len()
            ),
        ));
    }
    for (arg, expected_ty) in args.iter().zip(sig.param_types.iter().cloned()) {
        let actual_ty =
            infer_expression_type(arg, scope, memory_state, functions, structs, fn_ctx)?;
        if !can_assign(&expected_ty, &actual_ty) {
            return Err(sem_err(
                SEM_ARG_TYPE,
                format!(
                    "argument type mismatch for '{}': expected {:?}, got {:?}.",
                    name, expected_ty, actual_ty
                ),
            ));
        }
    }
    Ok(())
}

fn parse_declared_type_name(name: &str, structs: &HashMap<String, StructInfo>) -> ValueType {
    if let Some(inner) = name.strip_prefix("Task(").and_then(|s| s.strip_suffix(')')) {
        return ValueType::Task(Some(Box::new(parse_declared_type_name(
            inner.trim(),
            structs,
        ))));
    }
    if name == "Task" {
        return ValueType::Task(None);
    }
    if let Some(inner) = name
        .strip_prefix("Channel(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return ValueType::Channel(Box::new(parse_declared_type_name(inner.trim(), structs)));
    }
    if let Some(elem) = name.strip_suffix(" List") {
        let elem = elem.trim();
        let parsed_elem = parse_type_name(elem);
        if parsed_elem == ValueType::Unknown && structs.contains_key(elem) {
            return ValueType::List(Box::new(ValueType::Struct(elem.to_string())));
        }
        return ValueType::List(Box::new(parsed_elem));
    }
    let parsed = parse_type_name(name);
    if parsed == ValueType::Unknown && structs.contains_key(name) {
        ValueType::Struct(name.to_string())
    } else {
        parsed
    }
}

fn type_contains_memory(ty: &ValueType) -> bool {
    match ty {
        ValueType::Memory => true,
        ValueType::List(inner) => type_contains_memory(inner),
        ValueType::Task(Some(inner)) | ValueType::Channel(inner) => type_contains_memory(inner),
        _ => false,
    }
}

fn type_contains_task(ty: &ValueType) -> bool {
    match ty {
        ValueType::Task(_) => true,
        ValueType::List(inner) | ValueType::Channel(inner) => type_contains_task(inner),
        _ => false,
    }
}

fn type_contains_channel(ty: &ValueType) -> bool {
    match ty {
        ValueType::Channel(_) => true,
        ValueType::List(inner) => type_contains_channel(inner),
        ValueType::Task(Some(inner)) => type_contains_channel(inner),
        _ => false,
    }
}

fn is_value_safe_channel_message(ty: &ValueType, structs: &HashMap<String, StructInfo>) -> bool {
    if type_contains_memory(ty) || type_contains_task(ty) || type_contains_channel(ty) {
        return false;
    }
    match ty {
        ValueType::Struct(name) => structs
            .get(name)
            .map(|info| {
                info.fields
                    .values()
                    .all(|field_ty| is_value_safe_channel_message(field_ty, structs))
            })
            .unwrap_or(false),
        ValueType::List(_) => false,
        _ => true,
    }
}

fn is_task_safe_boundary_type(
    ty: &ValueType,
    structs: &HashMap<String, StructInfo>,
    allow_channel: bool,
) -> bool {
    match ty {
        ValueType::Int | ValueType::Float | ValueType::Bool | ValueType::Char | ValueType::Text => {
            true
        }
        ValueType::Struct(name) => structs
            .get(name)
            .map(|info| {
                info.fields
                    .values()
                    .all(|field_ty| is_task_safe_boundary_type(field_ty, structs, false))
            })
            .unwrap_or(false),
        ValueType::Channel(inner) => allow_channel && is_value_safe_channel_message(inner, structs),
        // Lists have mutable backing storage in the current runtime. Passing their
        // representation by value would create a cross-task mutable alias.
        ValueType::List(_) | ValueType::Memory | ValueType::Task(_) | ValueType::Unknown => false,
    }
}

fn ensure_memory_type_allowed(
    stmt: &Statement,
    ty: &ValueType,
    context: &str,
    allow_direct_memory: bool,
) -> Result<(), String> {
    if *ty == ValueType::Memory && allow_direct_memory {
        return Ok(());
    }
    if type_contains_memory(ty) {
        return Err(err_at_code(
            stmt,
            SEM_MEMORY_CAPABILITY,
            format!(
                "illegal Memory value usage: {} must not use Memory as a regular storable/returnable value.",
                context
            ),
        ));
    }
    Ok(())
}

fn ensure_task_type_allowed(
    stmt: &Statement,
    ty: &ValueType,
    context: &str,
    allow_direct_task: bool,
) -> Result<(), String> {
    if matches!(ty, ValueType::Task(_)) && allow_direct_task {
        return Ok(());
    }
    if type_contains_task(ty) {
        return Err(err_at_code(
            stmt,
            SEM_TASK_CAPABILITY,
            format!(
                "illegal Task value usage: {} must not use Task as a regular storable/returnable value.",
                context
            ),
        ));
    }
    Ok(())
}

fn ensure_channel_type_allowed(
    stmt: &Statement,
    ty: &ValueType,
    context: &str,
    allow_direct_channel: bool,
) -> Result<(), String> {
    if matches!(ty, ValueType::Channel(_)) && allow_direct_channel {
        return Ok(());
    }
    if type_contains_channel(ty) {
        return Err(err_at_code(
            stmt,
            SEM_CHANNEL_RULE,
            format!(
                "illegal Channel value usage: {} must not use Channel as a regular storable/returnable value.",
                context
            ),
        ));
    }
    Ok(())
}

fn is_region_relevant_type(
    ty: &ValueType,
    structs: &HashMap<String, StructInfo>,
    visiting: &mut Vec<String>,
) -> bool {
    match ty {
        ValueType::Text | ValueType::List(_) => true,
        ValueType::Struct(name) => {
            if visiting.iter().any(|item| item == name) {
                return false;
            }
            let Some(info) = structs.get(name) else {
                return false;
            };
            visiting.push(name.clone());
            let result = info
                .fields
                .values()
                .any(|field_ty| is_region_relevant_type(field_ty, structs, visiting));
            visiting.pop();
            result
        }
        _ => false,
    }
}

fn region_relevant(ty: &ValueType, structs: &HashMap<String, StructInfo>) -> bool {
    is_region_relevant_type(ty, structs, &mut Vec::new())
}

fn memory_is_external(memory_state: &MemoryState, memory_name: &str) -> bool {
    memory_state
        .memories
        .get(memory_name)
        .map(|binding| binding.is_external)
        .unwrap_or(false)
}

fn assign_memory_provenance(
    memory_state: &mut MemoryState,
    target: &str,
    ty: &ValueType,
    source_memory: Option<String>,
    structs: &HashMap<String, StructInfo>,
) {
    if region_relevant(ty, structs)
        && let Some(memory_name) = source_memory
    {
        if let Some(binding) = memory_state.memories.get_mut(&memory_name) {
            binding.is_cleared = false;
        }
        memory_state
            .variable_memory
            .insert(target.to_string(), memory_name);
        return;
    }
    memory_state.variable_memory.remove(target);
}

fn ensure_memory_sink_compatible(
    stmt: &Statement,
    memory_state: &MemoryState,
    sink_memory: Option<&str>,
    source_memory: Option<&str>,
) -> Result<(), String> {
    let Some(source_memory) = source_memory else {
        return Ok(());
    };
    if memory_is_external(memory_state, source_memory) {
        return Ok(());
    }
    if sink_memory == Some(source_memory) {
        return Ok(());
    }
    Err(err_at_code(
        stmt,
        SEM_MEMORY_RULE,
        format!(
            "memory escape is not allowed: region-owned value from '{}' cannot be stored into a longer-lived owner.",
            source_memory
        ),
    ))
}

#[allow(clippy::too_many_arguments)]
fn analyze_statements(
    statements: &[Statement],
    scope: &mut HashMap<String, ValueType>,
    memory_state: &mut MemoryState,
    functions: &HashMap<String, FunctionSig>,
    labels: &HashMap<String, Vec<String>>,
    structs: &HashMap<String, StructInfo>,
    task_context_functions: &HashSet<String>,
    fn_ctx: Option<FnContext>,
    in_loop: bool,
) -> Result<(), String> {
    for stmt in statements {
        analyze_statement(
            stmt,
            scope,
            memory_state,
            functions,
            labels,
            structs,
            task_context_functions,
            fn_ctx.clone(),
            in_loop,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn analyze_statement(
    stmt: &Statement,
    scope: &mut HashMap<String, ValueType>,
    memory_state: &mut MemoryState,
    functions: &HashMap<String, FunctionSig>,
    labels: &HashMap<String, Vec<String>>,
    structs: &HashMap<String, StructInfo>,
    task_context_functions: &HashSet<String>,
    fn_ctx: Option<FnContext>,
    in_loop: bool,
) -> Result<(), String> {
    match stmt {
        Statement::MemoryDecl {
            name,
            size_spec,
            on_error,
            ..
        } => {
            if scope.contains_key(name) {
                return Err(err_at_code(
                    stmt,
                    SEM_REDECLARATION,
                    format!(
                        "redeclaration in same scope: '{}' is already defined.",
                        name
                    ),
                ));
            }
            if size_spec.trim().is_empty() {
                return Err(err_at_code(
                    stmt,
                    SEM_MEMORY_RULE,
                    "memory(size) requires a non-empty size specification.".to_string(),
                ));
            }
            scope.insert(name.clone(), ValueType::Memory);
            memory_state.memories.insert(
                name.clone(),
                MemoryBinding {
                    is_external: false,
                    is_cleared: false,
                },
            );
            if let Some(on_error) = on_error {
                let mut on_error_scope = scope.clone();
                let mut on_error_memory = memory_state.clone();
                analyze_block(
                    on_error,
                    &mut on_error_scope,
                    &mut on_error_memory,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    fn_ctx,
                    in_loop,
                )?;
            }
            Ok(())
        }
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
                    format!(
                        "redeclaration in same scope: '{}' is already defined.",
                        name
                    ),
                ));
            }
            if contains_variable(value, name) {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_INIT,
                    format!(
                        "invalid initialization: '{}' is used in its own initializing expression.",
                        name
                    ),
                ));
            }
            let value_ty = infer_expression_type(
                value,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            let final_ty = if let Some(tn) = declared_type {
                let declared = parse_declared_type_name(tn, structs);
                if let ValueType::Channel(elem_ty) = &declared
                    && !is_value_safe_channel_message(elem_ty, structs)
                {
                    return Err(err_at_code(
                        stmt,
                        SEM_CHANNEL_RULE,
                        format!(
                            "Channel message type must be value-safe, got {:?}.",
                            elem_ty
                        ),
                    ));
                }
                ensure_memory_type_allowed(stmt, &declared, "variable declaration type", false)?;
                ensure_task_type_allowed(
                    stmt,
                    &declared,
                    "variable declaration type",
                    matches!(declared, ValueType::Task(_)),
                )?;
                ensure_channel_type_allowed(
                    stmt,
                    &declared,
                    "variable declaration type",
                    matches!(declared, ValueType::Channel(_)),
                )?;
                if !can_assign(&declared, &value_ty) {
                    return Err(err_at_code(
                        stmt,
                        SEM_TYPE_MISMATCH,
                        format!(
                            "type mismatch in declaration '{}': cannot assign {:?} to {:?}.",
                            name, value_ty, declared
                        ),
                    ));
                }
                declared
            } else {
                value_ty
            };
            ensure_memory_type_allowed(stmt, &final_ty, "variable declaration value", false)?;
            ensure_task_type_allowed(
                stmt,
                &final_ty,
                "variable declaration value",
                matches!(final_ty, ValueType::Task(_)),
            )?;
            ensure_channel_type_allowed(
                stmt,
                &final_ty,
                "variable declaration value",
                matches!(final_ty, ValueType::Channel(_)),
            )?;
            let source_memory = infer_expression_memory_provenance(
                value,
                &final_ty,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?
            .or_else(|| {
                if region_relevant(&final_ty, structs) {
                    memory_state.active_memory.clone()
                } else {
                    None
                }
            });
            scope.insert(name.clone(), final_ty);
            let final_ty = scope.get(name).cloned().unwrap_or(ValueType::Unknown);
            if matches!(final_ty, ValueType::Channel(_)) && in_loop {
                return Err(err_at_code(
                    stmt,
                    SEM_CHANNEL_RULE,
                    "Channel owner cannot be created inside a loop because break/continue could bypass deterministic cleanup. Create it in the enclosing scope."
                        .to_string(),
                ));
            }
            if matches!(final_ty, ValueType::Channel(_)) && memory_state.active_memory.is_some() {
                return Err(err_at_code(
                    stmt,
                    SEM_CHANNEL_RULE,
                    "Channel owner cannot be created inside 'place in' because recovery control flow could bypass deterministic cleanup. Create it outside the memory region."
                        .to_string(),
                ));
            }
            if let ValueType::Task(result_type) = &final_ty {
                memory_state.tasks.insert(
                    name.clone(),
                    TaskBinding {
                        result_type: result_type.as_ref().map(|ty| (**ty).clone()),
                        waited: false,
                        stopped: false,
                    },
                );
            }
            if let ValueType::Channel(elem_ty) = &final_ty {
                memory_state
                    .channels
                    .insert(name.clone(), (**elem_ty).clone());
            }
            record_task_effects_in_expr(stmt, value, scope, memory_state)?;
            assign_memory_provenance(memory_state, name, &final_ty, source_memory, structs);
            Ok(())
        }
        Statement::Assignment { target, value, .. } => {
            let Some(target_ty) = scope.get(target).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        target
                    ),
                ));
            };
            if target_ty == ValueType::Memory {
                return Err(err_at_code(
                    stmt,
                    SEM_MEMORY_CAPABILITY,
                    format!(
                        "illegal Memory value usage: '{}' cannot be reassigned or copied as a regular value.",
                        target
                    ),
                ));
            }
            if matches!(target_ty, ValueType::Task(_) | ValueType::Channel(_)) {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_CAPABILITY,
                    format!(
                        "illegal Task/Channel value usage: '{}' cannot be reassigned or copied as a regular value.",
                        target
                    ),
                ));
            }
            let value_ty = infer_expression_type(
                value,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            if !can_assign(&target_ty, &value_ty) {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    format!(
                        "type mismatch in assignment to '{}': cannot assign {:?} to {:?}.",
                        target, value_ty, target_ty
                    ),
                ));
            }
            let source_memory = infer_expression_memory_provenance(
                value,
                &target_ty,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            ensure_memory_sink_compatible(
                stmt,
                memory_state,
                memory_state.variable_memory.get(target).map(String::as_str),
                source_memory.as_deref(),
            )?;
            record_task_effects_in_expr(stmt, value, scope, memory_state)?;
            assign_memory_provenance(memory_state, target, &target_ty, source_memory, structs);
            Ok(())
        }
        Statement::IncDec {
            target,
            is_increment: _,
            ..
        } => {
            let Some(target_ty) = scope.get(target).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        target
                    ),
                ));
            };
            match target_ty {
                ValueType::Int | ValueType::Float => Ok(()),
                other => Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    format!(
                        "increment/decrement requires numeric variable, got {:?}.",
                        other
                    ),
                )),
            }
        }
        Statement::FieldAssignment {
            object,
            field,
            value,
            ..
        } => {
            let owner_ty = if object == "my" {
                let Some(ctx) = fn_ctx.as_ref() else {
                    return Err(err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
                        "my is only allowed inside struct methods.".to_string(),
                    ));
                };
                let Some(self_name) = ctx.self_struct.as_ref() else {
                    return Err(err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
                        "my is only allowed inside struct methods.".to_string(),
                    ));
                };
                ValueType::Struct(self_name.clone())
            } else if let Some(v) = scope.get(object).cloned() {
                v
            } else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        object
                    ),
                ));
            };
            let ValueType::Struct(owner) = owner_ty else {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    format!(
                        "field assignment requires struct receiver, got {:?}.",
                        owner_ty
                    ),
                ));
            };
            let Some(info) = structs.get(&owner) else {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    format!("unknown struct type '{}'.", owner),
                ));
            };
            let Some(field_ty) = info.fields.get(field).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    format!("unknown field '{}.{}'.", owner, field),
                ));
            };
            let value_ty = infer_expression_type(
                value,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            if !can_assign(&field_ty, &value_ty) {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    format!(
                        "type mismatch in field assignment '{}.{}': cannot assign {:?} to {:?}.",
                        owner, field, value_ty, field_ty
                    ),
                ));
            }
            let source_memory = infer_expression_memory_provenance(
                value,
                &field_ty,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            let sink_memory = if object == "my" {
                None
            } else {
                memory_state.variable_memory.get(object).map(String::as_str)
            };
            ensure_memory_sink_compatible(
                stmt,
                memory_state,
                sink_memory,
                source_memory.as_deref(),
            )?;
            Ok(())
        }
        Statement::FunctionDef {
            name, params, body, ..
        } => {
            let mut fn_scope = scope.clone();
            let mut fn_memory_state = MemoryState::default();
            for p in params {
                let pty = param_type_or_default(p);
                fn_scope.insert(p.name.clone(), pty.clone());
                if pty == ValueType::Memory {
                    fn_memory_state.memories.insert(
                        p.name.clone(),
                        MemoryBinding {
                            is_external: true,
                            is_cleared: false,
                        },
                    );
                }
                if let ValueType::Channel(elem_ty) = &pty {
                    fn_memory_state
                        .channels
                        .insert(p.name.clone(), (**elem_ty).clone());
                }
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
                self_struct: None,
                is_task_context: task_context_functions.contains(name),
            };
            analyze_block(
                body,
                &mut fn_scope,
                &mut fn_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                Some(local_ctx),
                false,
            )?;
            if sig.is_danger && !block_guarantees_termination(body) {
                return Err(err_at_code(
                    stmt,
                    SEM_RETURN_RULE,
                    format!(
                        "danger fn '{}' must end with explicit return/return error on all paths.",
                        name
                    ),
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
            let cty = infer_expression_type(
                condition,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            if cty != ValueType::Bool {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    "if condition must be bool.".to_string(),
                ));
            }
            let mut then_scope = scope.clone();
            let mut then_memory_state = memory_state.clone();
            analyze_block(
                then_block,
                &mut then_scope,
                &mut then_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx.clone(),
                in_loop,
            )?;
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                let mut else_memory_state = memory_state.clone();
                analyze_block(
                    else_block,
                    &mut else_scope,
                    &mut else_memory_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    fn_ctx.clone(),
                    in_loop,
                )?;
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
            let mut loop_memory_state = memory_state.clone();
            if let Some(init) = initialization {
                if let Expression::VariableReference(name) = init.as_ref() {
                    let inferred_item_ty = if let Some(coll) = condition {
                        match infer_expression_type(
                            coll,
                            &loop_scope,
                            &loop_memory_state,
                            functions,
                            structs,
                            fn_ctx.as_ref(),
                        )? {
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
                                ));
                            }
                        }
                    } else {
                        ValueType::Unknown
                    };
                    loop_scope.insert(name.clone(), inferred_item_ty);
                } else {
                    let _ = infer_expression_type(
                        init,
                        &loop_scope,
                        &loop_memory_state,
                        functions,
                        structs,
                        fn_ctx.as_ref(),
                    )?;
                }
            }
            if let Some(cond) = condition {
                let _ = infer_expression_type(
                    cond,
                    &loop_scope,
                    &loop_memory_state,
                    functions,
                    structs,
                    fn_ctx.as_ref(),
                )?;
            }
            if let Some(upd) = update {
                let _ = infer_expression_type(
                    upd,
                    &loop_scope,
                    &loop_memory_state,
                    functions,
                    structs,
                    fn_ctx.as_ref(),
                )?;
            }
            analyze_block(
                body,
                &mut loop_scope,
                &mut loop_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx,
                true,
            )
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            let when_ty = infer_expression_type(
                when_expression,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            for (case_exprs, block) in cases {
                for expr in case_exprs {
                    let case_ty = infer_expression_type(
                        expr,
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx.as_ref(),
                    )?;
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
                let mut case_memory_state = memory_state.clone();
                analyze_block(
                    block,
                    &mut case_scope,
                    &mut case_memory_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    fn_ctx.clone(),
                    in_loop,
                )?;
            }
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                let mut else_memory_state = memory_state.clone();
                analyze_block(
                    else_block,
                    &mut else_scope,
                    &mut else_memory_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    fn_ctx.clone(),
                    in_loop,
                )?;
            }
            Ok(())
        }
        Statement::WhileLoop {
            condition, body, ..
        } => {
            let cty = infer_expression_type(
                condition,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            if cty != ValueType::Bool {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    "while condition must be bool.".to_string(),
                ));
            }
            let mut while_scope = scope.clone();
            let mut while_memory_state = memory_state.clone();
            analyze_block(
                body,
                &mut while_scope,
                &mut while_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx,
                true,
            )
        }
        Statement::LoopStatement { body, .. } => {
            let mut local_scope = scope.clone();
            let mut local_memory_state = memory_state.clone();
            analyze_block(
                body,
                &mut local_scope,
                &mut local_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx,
                true,
            )
        }
        Statement::BreakStatement { .. } | Statement::ContinueStatement { .. } => {
            if !in_loop {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    "break/continue are allowed only inside loops.".to_string(),
                ));
            }
            Ok(())
        }
        Statement::PassStatement { .. } => Ok(()),
        Statement::OnErrorBlock { statements, .. }
        | Statement::BlockStatement { statements, .. } => {
            let mut local_scope = scope.clone();
            let mut local_memory_state = memory_state.clone();
            analyze_statements(
                statements,
                &mut local_scope,
                &mut local_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx,
                in_loop,
            )
        }
        Statement::PlaceIn {
            memory_name,
            on_error,
            body,
            ..
        } => {
            let Some(memory_ty) = scope.get(memory_name).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        memory_name
                    ),
                ));
            };
            if memory_ty != ValueType::Memory {
                return Err(err_at_code(
                    stmt,
                    SEM_MEMORY_RULE,
                    format!("place in expects Memory target, got {:?}.", memory_ty),
                ));
            }
            if !memory_state.memories.contains_key(memory_name) {
                return Err(err_at_code(
                    stmt,
                    SEM_MEMORY_RULE,
                    format!(
                        "memory '{}' is not available in current function context.",
                        memory_name
                    ),
                ));
            }
            if memory_state.active_memory.as_deref() == Some(memory_name.as_str()) {
                return Err(err_at_code(
                    stmt,
                    SEM_MEMORY_RULE,
                    format!(
                        "nested place in same Memory is forbidden: '{}'. Reuse the current placement block or switch to a different scratch region.",
                        memory_name
                    ),
                ));
            }
            let mut place_scope = scope.clone();
            let mut place_memory_state = memory_state.clone();
            place_memory_state.active_memory = Some(memory_name.clone());
            analyze_block(
                body,
                &mut place_scope,
                &mut place_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx.clone(),
                in_loop,
            )?;
            for name in scope.keys() {
                if let Some(memory_name) = place_memory_state.variable_memory.get(name).cloned() {
                    memory_state
                        .variable_memory
                        .insert(name.clone(), memory_name);
                } else {
                    memory_state.variable_memory.remove(name);
                }
            }
            memory_state.memories = place_memory_state.memories;
            if let Some(on_error) = on_error {
                let mut on_error_scope = scope.clone();
                let mut on_error_memory = memory_state.clone();
                analyze_block(
                    on_error,
                    &mut on_error_scope,
                    &mut on_error_memory,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    fn_ctx,
                    in_loop,
                )?;
            }
            Ok(())
        }
        Statement::MemoryClear { memory_name, .. } => {
            let Some(memory_ty) = scope.get(memory_name).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        memory_name
                    ),
                ));
            };
            if memory_ty != ValueType::Memory {
                return Err(err_at_code(
                    stmt,
                    SEM_MEMORY_RULE,
                    format!(
                        "clear() is only allowed on Memory values, got {:?}.",
                        memory_ty
                    ),
                ));
            }
            if memory_state.active_memory.as_deref() == Some(memory_name.as_str()) {
                return Err(err_at_code(
                    stmt,
                    SEM_MEMORY_RULE,
                    format!(
                        "forbidden in-block clear: '{}.clear()' is not allowed inside an active 'place in {} {{ ... }}' block. Clear the region after the block or in the trailing on error handler.",
                        memory_name, memory_name
                    ),
                ));
            }
            let Some(binding) = memory_state.memories.get_mut(memory_name) else {
                return Err(err_at_code(
                    stmt,
                    SEM_MEMORY_RULE,
                    format!(
                        "memory '{}' is not available in current function context.",
                        memory_name
                    ),
                ));
            };
            binding.is_cleared = true;
            Ok(())
        }
        Statement::OnBlock { trigger, body, .. } => {
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
            let mut local_scope = scope.clone();
            let mut local_memory_state = memory_state.clone();
            analyze_block(
                body,
                &mut local_scope,
                &mut local_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx,
                in_loop,
            )
        }
        Statement::StopTask { task_name, .. } => {
            let Some(task_ty) = scope.get(task_name).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        task_name
                    ),
                ));
            };
            if !matches!(task_ty, ValueType::Task(_)) {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_RULE,
                    format!("stop expects Task handle, got {:?}.", task_ty),
                ));
            }
            let Some(binding) = memory_state.tasks.get_mut(task_name) else {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_RULE,
                    format!(
                        "task handle '{}' is not available in current scope.",
                        task_name
                    ),
                ));
            };
            if binding.stopped {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_RULE,
                    format!("task handle '{}' was already stopped.", task_name),
                ));
            }
            if binding.waited {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_RULE,
                    format!("task handle '{}' was already waited.", task_name),
                ));
            }
            binding.stopped = true;
            Ok(())
        }
        Statement::DangerAssignOnError {
            target,
            call_name,
            args,
            on_error,
            ..
        } => {
            if builtin_from_name(call_name).is_some() {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!(
                        "on error requires danger fn call: builtin '{}' is not danger in v1.",
                        call_name
                    ),
                ));
            }
            let Some(sig) = functions.get(call_name) else {
                return Err(err_at_code(
                    stmt,
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in on error call.", call_name),
                ));
            };
            if !sig.is_danger {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!(
                        "on error requires danger fn call: '{}' is not declared as danger.",
                        call_name
                    ),
                ));
            }
            if !scope.contains_key(target) {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        target
                    ),
                ));
            }
            validate_call_args(
                call_name,
                args,
                sig,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            let mut on_error_scope = scope.clone();
            let mut on_error_memory_state = memory_state.clone();
            analyze_block(
                on_error,
                &mut on_error_scope,
                &mut on_error_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx,
                in_loop,
            )
        }
        Statement::DangerCallOnError {
            call_name,
            args,
            on_error,
            ..
        } => {
            if builtin_from_name(call_name).is_some() {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!(
                        "on error requires danger fn call: builtin '{}' is not danger in v1.",
                        call_name
                    ),
                ));
            }
            let Some(sig) = functions.get(call_name) else {
                return Err(err_at_code(
                    stmt,
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in on error call.", call_name),
                ));
            };
            if !sig.is_danger {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!(
                        "on error requires danger fn call: '{}' is not declared as danger.",
                        call_name
                    ),
                ));
            }
            validate_call_args(
                call_name,
                args,
                sig,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            let mut on_error_scope = scope.clone();
            let mut on_error_memory_state = memory_state.clone();
            analyze_block(
                on_error,
                &mut on_error_scope,
                &mut on_error_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx,
                in_loop,
            )
        }
        Statement::ListPush {
            list_name, value, ..
        } => {
            let Some(list_ty) = scope.get(list_name).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        list_name
                    ),
                ));
            };
            let value_ty = infer_expression_type(
                value,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
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
                    let source_memory = infer_expression_memory_provenance(
                        value,
                        &value_ty,
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx.as_ref(),
                    )?;
                    ensure_memory_sink_compatible(
                        stmt,
                        memory_state,
                        memory_state
                            .variable_memory
                            .get(list_name)
                            .map(String::as_str),
                        source_memory.as_deref(),
                    )?;
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
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        target
                    ),
                ));
            };
            let Some(list_ty) = scope.get(list_name).cloned() else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        list_name
                    ),
                ));
            };
            let elem_ty = match list_ty {
                ValueType::List(elem_ty) => (*elem_ty).clone(),
                other => {
                    return Err(err_at_code(
                        stmt,
                        SEM_TYPE_MISMATCH,
                        format!("pop is supported only for List, got {:?}.", other),
                    ));
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
            ensure_memory_sink_compatible(
                stmt,
                memory_state,
                memory_state.variable_memory.get(target).map(String::as_str),
                memory_state
                    .variable_memory
                    .get(list_name)
                    .map(String::as_str),
            )?;
            assign_memory_provenance(
                memory_state,
                target,
                &target_ty,
                memory_state.variable_memory.get(list_name).cloned(),
                structs,
            );
            let mut on_error_scope = scope.clone();
            let mut on_error_memory_state = memory_state.clone();
            analyze_block(
                on_error,
                &mut on_error_scope,
                &mut on_error_memory_state,
                functions,
                labels,
                structs,
                task_context_functions,
                fn_ctx,
                in_loop,
            )
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
            if let Some(ref ctx) = fn_ctx {
                if let Some(expr) = value {
                    let actual = infer_expression_type(
                        expr,
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx.as_ref(),
                    )?;
                    let source_memory = infer_expression_memory_provenance(
                        expr,
                        &actual,
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx.as_ref(),
                    )?;
                    if actual == ValueType::Memory {
                        return Err(err_at_code(
                            stmt,
                            SEM_MEMORY_CAPABILITY,
                            "illegal Memory value usage: Memory handles cannot be returned from functions."
                                .to_string(),
                        ));
                    }
                    if matches!(actual, ValueType::Task(_)) {
                        return Err(err_at_code(
                            stmt,
                            SEM_TASK_CAPABILITY,
                            "illegal Task value usage: Task handles cannot be returned from functions."
                                .to_string(),
                        ));
                    }
                    if matches!(actual, ValueType::Channel(_)) {
                        return Err(err_at_code(
                            stmt,
                            SEM_CHANNEL_RULE,
                            "illegal Channel value usage: Channel handles cannot be returned from functions."
                                .to_string(),
                        ));
                    }
                    if let Some(memory_name) = source_memory
                        && !memory_is_external(memory_state, &memory_name)
                    {
                        return Err(err_at_code(
                            stmt,
                            SEM_MEMORY_LIFETIME,
                            format!(
                                "cannot return region-owned value from local Memory '{}'. Pass Memory into the function to return values allocated in it.",
                                memory_name
                            ),
                        ));
                    }
                    if let Some(expected) = ctx.return_type.as_ref()
                        && !can_assign(expected, &actual)
                    {
                        return Err(err_at_code(
                            stmt,
                            SEM_TYPE_MISMATCH,
                            format!(
                                "type mismatch in return: cannot return {:?} where {:?} expected.",
                                actual, expected
                            ),
                        ));
                    }
                    record_task_effects_in_expr(stmt, expr, scope, memory_state)?;
                } else if !ctx.is_danger && ctx.return_type.is_some() {
                    return Err(err_at_code(
                        stmt,
                        SEM_RETURN_RULE,
                        "non-danger function with return type must return a value.".to_string(),
                    ));
                }
            } else if let Some(expr) = value {
                let _ = infer_expression_type(
                    expr,
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx.as_ref(),
                )?;
                record_task_effects_in_expr(stmt, expr, scope, memory_state)?;
            }
            Ok(())
        }
        Statement::ExpressionStatement { expr, .. } => {
            if matches!(expr.as_ref(), Expression::RunTask { .. }) {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_RULE,
                    "task handle ignored: assign the result of 'run' to a Task handle and wait it on all paths."
                        .to_string(),
                ));
            }
            let _ = infer_expression_type(
                expr,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            record_task_effects_in_expr(stmt, expr, scope, memory_state)?;
            Ok(())
        }
        Statement::StructDecl { name, methods, .. } => {
            for m in methods {
                let mut method_scope = scope.clone();
                let mut method_memory_state = MemoryState::default();
                method_scope.insert("my".to_string(), ValueType::Struct(name.clone()));
                for p in &m.params {
                    let pty = param_type_or_default(p);
                    method_scope.insert(p.name.clone(), pty.clone());
                    if pty == ValueType::Memory {
                        method_memory_state.memories.insert(
                            p.name.clone(),
                            MemoryBinding {
                                is_external: true,
                                is_cleared: false,
                            },
                        );
                    }
                    if let ValueType::Channel(elem_ty) = &pty {
                        method_memory_state
                            .channels
                            .insert(p.name.clone(), (**elem_ty).clone());
                    }
                }
                let method_ctx = FnContext {
                    is_danger: m.is_danger,
                    return_type: m
                        .returns
                        .as_deref()
                        .map(parse_type_name)
                        .or(Some(ValueType::Int)),
                    self_struct: Some(name.clone()),
                    is_task_context: false,
                };
                analyze_block(
                    &m.body,
                    &mut method_scope,
                    &mut method_memory_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    Some(method_ctx),
                    false,
                )?;
            }
            Ok(())
        }
        Statement::LabelDecl { .. } => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
fn analyze_block(
    block: &BlockStatement,
    scope: &mut HashMap<String, ValueType>,
    memory_state: &mut MemoryState,
    functions: &HashMap<String, FunctionSig>,
    labels: &HashMap<String, Vec<String>>,
    structs: &HashMap<String, StructInfo>,
    task_context_functions: &HashSet<String>,
    fn_ctx: Option<FnContext>,
    in_loop: bool,
) -> Result<(), String> {
    analyze_statements(
        &block.statements,
        scope,
        memory_state,
        functions,
        labels,
        structs,
        task_context_functions,
        fn_ctx,
        in_loop,
    )
}

fn param_type_or_default(param: &FunctionParam) -> ValueType {
    param
        .param_type
        .as_deref()
        .map(parse_type_name)
        .unwrap_or(ValueType::Int)
}

fn record_task_effects_in_expr(
    stmt: &Statement,
    expr: &Expression,
    scope: &HashMap<String, ValueType>,
    memory_state: &mut MemoryState,
) -> Result<(), String> {
    match expr {
        Expression::WaitTask { task_name } => {
            let Some(task_ty) = scope.get(task_name) else {
                return Err(err_at_code(
                    stmt,
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        task_name
                    ),
                ));
            };
            if !matches!(task_ty, ValueType::Task(_)) {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_RULE,
                    format!("wait expects Task handle, got {:?}.", task_ty),
                ));
            }
            let Some(binding) = memory_state.tasks.get_mut(task_name) else {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_RULE,
                    format!(
                        "task handle '{}' is not available in current scope.",
                        task_name
                    ),
                ));
            };
            if binding.waited {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_RULE,
                    format!("task handle '{}' was already waited.", task_name),
                ));
            }
            binding.waited = true;
            Ok(())
        }
        Expression::RunTask { args, .. } | Expression::Call { args, .. } => {
            for arg in args {
                record_task_effects_in_expr(stmt, arg, scope, memory_state)?;
            }
            Ok(())
        }
        Expression::BinaryOp { left, right, .. } => {
            record_task_effects_in_expr(stmt, left, scope, memory_state)?;
            if let Some(right) = right {
                record_task_effects_in_expr(stmt, right, scope, memory_state)?;
            }
            Ok(())
        }
        Expression::Index { base, index } => {
            record_task_effects_in_expr(stmt, base, scope, memory_state)?;
            record_task_effects_in_expr(stmt, index, scope, memory_state)
        }
        Expression::ListLiteral(items) => {
            for item in items {
                record_task_effects_in_expr(stmt, item, scope, memory_state)?;
            }
            Ok(())
        }
        Expression::StructConstruction { fields } => {
            for value in fields.values() {
                record_task_effects_in_expr(stmt, value, scope, memory_state)?;
            }
            Ok(())
        }
        Expression::VariableReference(_)
        | Expression::MemberAccess { .. }
        | Expression::Stopping
        | Expression::LiteralInt(_)
        | Expression::LiteralFloat(_)
        | Expression::LiteralBool(_)
        | Expression::LiteralString(_) => Ok(()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TaskFlowBinding {
    stopped: bool,
    line: u32,
    column: u32,
}

type TaskFlowState = HashMap<String, TaskFlowBinding>;

fn open_task_error(name: &str, binding: &TaskFlowBinding) -> String {
    format_diagnostic(
        DiagnosticKind::Semantic,
        Some(SEM_TASK_RULE),
        format!(
            "task handle '{}' must be waited on all paths before leaving its owning scope.",
            name
        ),
        Some(binding.line),
        Some(binding.column),
        None,
    )
}

fn ensure_no_new_task_bindings(
    initial: &TaskFlowState,
    outputs: &[TaskFlowState],
) -> Result<(), String> {
    for output in outputs {
        for (name, binding) in output {
            if !initial.contains_key(name) {
                return Err(open_task_error(name, binding));
            }
        }
    }
    Ok(())
}

fn apply_task_flow_expression(
    stmt: &Statement,
    expr: &Expression,
    state: &mut TaskFlowState,
) -> Result<(), String> {
    match expr {
        Expression::RunTask { .. } => Err(err_at_code(
            stmt,
            SEM_TASK_RULE,
            "task handle ignored: 'run' is only allowed as the initializer of an owning Task declaration."
                .to_string(),
        )),
        Expression::WaitTask { task_name } => {
            if state.remove(task_name).is_none() {
                return Err(err_at_code(
                    stmt,
                    SEM_TASK_RULE,
                    format!(
                        "task handle '{}' is not live on every path at this wait.",
                        task_name
                    ),
                ));
            }
            Ok(())
        }
        Expression::Call { args, .. } | Expression::ListLiteral(args) => {
            for arg in args {
                apply_task_flow_expression(stmt, arg, state)?;
            }
            Ok(())
        }
        Expression::Index { base, index } => {
            apply_task_flow_expression(stmt, base, state)?;
            apply_task_flow_expression(stmt, index, state)
        }
        Expression::BinaryOp { left, right, .. } => {
            apply_task_flow_expression(stmt, left, state)?;
            if let Some(right) = right {
                apply_task_flow_expression(stmt, right, state)?;
            }
            Ok(())
        }
        Expression::StructConstruction { fields } => {
            for value in fields.values() {
                apply_task_flow_expression(stmt, value, state)?;
            }
            Ok(())
        }
        Expression::LiteralInt(_)
        | Expression::LiteralFloat(_)
        | Expression::LiteralBool(_)
        | Expression::LiteralString(_)
        | Expression::VariableReference(_)
        | Expression::MemberAccess { .. }
        | Expression::Stopping => Ok(()),
    }
}

fn apply_task_flow_expressions(
    stmt: &Statement,
    expressions: &[Expression],
    state: &mut TaskFlowState,
) -> Result<(), String> {
    for expression in expressions {
        apply_task_flow_expression(stmt, expression, state)?;
    }
    Ok(())
}

fn expression_uses_task_handle(expr: &Expression, names: &HashSet<String>) -> bool {
    match expr {
        Expression::WaitTask { task_name } => names.contains(task_name),
        Expression::Call { args, .. }
        | Expression::RunTask { args, .. }
        | Expression::ListLiteral(args) => args
            .iter()
            .any(|arg| expression_uses_task_handle(arg, names)),
        Expression::Index { base, index } => {
            expression_uses_task_handle(base, names) || expression_uses_task_handle(index, names)
        }
        Expression::BinaryOp { left, right, .. } => {
            expression_uses_task_handle(left, names)
                || right
                    .as_deref()
                    .map(|expr| expression_uses_task_handle(expr, names))
                    .unwrap_or(false)
        }
        Expression::StructConstruction { fields } => fields
            .values()
            .any(|value| expression_uses_task_handle(value, names)),
        _ => false,
    }
}

fn statements_use_task_handle(statements: &[Statement], names: &HashSet<String>) -> bool {
    statements.iter().any(|stmt| match stmt {
        Statement::StopTask { task_name, .. } => names.contains(task_name),
        Statement::VarDecl { value, .. }
        | Statement::Assignment { value, .. }
        | Statement::FieldAssignment { value, .. }
        | Statement::ListPush { value, .. }
        | Statement::ExpressionStatement { expr: value, .. } => {
            expression_uses_task_handle(value, names)
        }
        Statement::ReturnStatement {
            value: Some(value), ..
        } => expression_uses_task_handle(value, names),
        Statement::IfStatement {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_uses_task_handle(condition, names)
                || statements_use_task_handle(&then_block.statements, names)
                || else_block
                    .as_deref()
                    .map(|block| statements_use_task_handle(&block.statements, names))
                    .unwrap_or(false)
        }
        Statement::ForLoop { body, .. }
        | Statement::WhileLoop { body, .. }
        | Statement::LoopStatement { body, .. }
        | Statement::OnBlock { body, .. } => statements_use_task_handle(&body.statements, names),
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            expression_uses_task_handle(when_expression, names)
                || cases
                    .iter()
                    .any(|(_, block)| statements_use_task_handle(&block.statements, names))
                || else_block
                    .as_deref()
                    .map(|block| statements_use_task_handle(&block.statements, names))
                    .unwrap_or(false)
        }
        Statement::PlaceIn { body, on_error, .. } => {
            statements_use_task_handle(&body.statements, names)
                || on_error
                    .as_deref()
                    .map(|block| statements_use_task_handle(&block.statements, names))
                    .unwrap_or(false)
        }
        Statement::MemoryDecl { on_error, .. } => on_error
            .as_deref()
            .map(|block| statements_use_task_handle(&block.statements, names))
            .unwrap_or(false),
        Statement::DangerAssignOnError { args, on_error, .. }
        | Statement::DangerCallOnError { args, on_error, .. } => {
            args.iter()
                .any(|arg| expression_uses_task_handle(arg, names))
                || statements_use_task_handle(&on_error.statements, names)
        }
        Statement::ListPopOnError { on_error, .. } => {
            statements_use_task_handle(&on_error.statements, names)
        }
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => {
            statements_use_task_handle(statements, names)
        }
        Statement::StructDecl { methods, .. } => methods
            .iter()
            .any(|method| statements_use_task_handle(&method.body.statements, names)),
        _ => false,
    })
}

fn validate_loop_task_flow(
    stmt: &Statement,
    body: &BlockStatement,
    states: Vec<TaskFlowState>,
) -> Result<Vec<TaskFlowState>, String> {
    let mut outputs = Vec::new();
    for state in states {
        let outer_names = state.keys().cloned().collect::<HashSet<_>>();
        if statements_use_task_handle(&body.statements, &outer_names) {
            return Err(err_at_code(
                stmt,
                SEM_TASK_RULE,
                "task lifecycle cannot depend on a loop iteration; create, stop and wait the task outside the loop, or complete its full lifecycle inside one iteration."
                    .to_string(),
            ));
        }
        let body_outputs = process_task_flow_statements(&body.statements, vec![state.clone()])?;
        ensure_no_new_task_bindings(&state, &body_outputs)?;
        if body_outputs.iter().any(|output| output != &state) {
            return Err(err_at_code(
                stmt,
                SEM_TASK_RULE,
                "task lifecycle cannot depend on a loop iteration; create, stop and wait the task outside the loop, or complete its full lifecycle inside one iteration."
                    .to_string(),
            ));
        }
        outputs.push(state);
    }
    Ok(outputs)
}

fn process_task_flow_statement(
    stmt: &Statement,
    states: Vec<TaskFlowState>,
) -> Result<Vec<TaskFlowState>, String> {
    match stmt {
        Statement::VarDecl {
            name, value, loc, ..
        } => {
            let mut outputs = Vec::new();
            for mut state in states {
                if let Expression::RunTask { args, .. } = value.as_ref() {
                    apply_task_flow_expressions(stmt, args, &mut state)?;
                    if state.contains_key(name) {
                        return Err(err_at_code(
                            stmt,
                            SEM_TASK_RULE,
                            format!("task handle '{}' already has an active owner.", name),
                        ));
                    }
                    state.insert(
                        name.clone(),
                        TaskFlowBinding {
                            stopped: false,
                            line: loc.line,
                            column: loc.column,
                        },
                    );
                } else {
                    apply_task_flow_expression(stmt, value, &mut state)?;
                }
                outputs.push(state);
            }
            Ok(outputs)
        }
        Statement::StopTask { task_name, .. } => {
            let mut outputs = Vec::new();
            for mut state in states {
                let Some(binding) = state.get_mut(task_name) else {
                    return Err(err_at_code(
                        stmt,
                        SEM_TASK_RULE,
                        format!(
                            "task handle '{}' is not live on every path at this stop.",
                            task_name
                        ),
                    ));
                };
                if binding.stopped {
                    return Err(err_at_code(
                        stmt,
                        SEM_TASK_RULE,
                        format!("task handle '{}' was already stopped.", task_name),
                    ));
                }
                binding.stopped = true;
                outputs.push(state);
            }
            Ok(outputs)
        }
        Statement::ExpressionStatement { expr, .. } => {
            let mut outputs = Vec::new();
            for mut state in states {
                apply_task_flow_expression(stmt, expr, &mut state)?;
                outputs.push(state);
            }
            Ok(outputs)
        }
        Statement::Assignment { value, .. }
        | Statement::FieldAssignment { value, .. }
        | Statement::ListPush { value, .. } => {
            let mut outputs = Vec::new();
            for mut state in states {
                apply_task_flow_expression(stmt, value, &mut state)?;
                outputs.push(state);
            }
            Ok(outputs)
        }
        Statement::ReturnStatement { value, .. } => {
            for mut state in states {
                if let Some(value) = value {
                    apply_task_flow_expression(stmt, value, &mut state)?;
                }
                if let Some((name, binding)) = state.iter().next() {
                    return Err(open_task_error(name, binding));
                }
            }
            Ok(Vec::new())
        }
        Statement::ReturnError { .. } => {
            for state in states {
                if let Some((name, binding)) = state.iter().next() {
                    return Err(open_task_error(name, binding));
                }
            }
            Ok(Vec::new())
        }
        Statement::IfStatement {
            condition,
            then_block,
            else_block,
            ..
        } => {
            let mut outputs = Vec::new();
            for mut state in states {
                apply_task_flow_expression(stmt, condition, &mut state)?;
                let then_outputs =
                    process_task_flow_statements(&then_block.statements, vec![state.clone()])?;
                ensure_no_new_task_bindings(&state, &then_outputs)?;
                outputs.extend(then_outputs);
                if let Some(else_block) = else_block {
                    let else_outputs =
                        process_task_flow_statements(&else_block.statements, vec![state.clone()])?;
                    ensure_no_new_task_bindings(&state, &else_outputs)?;
                    outputs.extend(else_outputs);
                } else {
                    outputs.push(state);
                }
            }
            Ok(outputs)
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            let mut outputs = Vec::new();
            for mut state in states {
                apply_task_flow_expression(stmt, when_expression, &mut state)?;
                for (_, block) in cases {
                    let case_outputs =
                        process_task_flow_statements(&block.statements, vec![state.clone()])?;
                    ensure_no_new_task_bindings(&state, &case_outputs)?;
                    outputs.extend(case_outputs);
                }
                if let Some(else_block) = else_block {
                    let else_outputs =
                        process_task_flow_statements(&else_block.statements, vec![state.clone()])?;
                    ensure_no_new_task_bindings(&state, &else_outputs)?;
                    outputs.extend(else_outputs);
                } else {
                    outputs.push(state);
                }
            }
            Ok(outputs)
        }
        Statement::ForLoop { body, .. }
        | Statement::WhileLoop { body, .. }
        | Statement::LoopStatement { body, .. } => validate_loop_task_flow(stmt, body, states),
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => {
            let mut outputs = Vec::new();
            for state in states {
                let block_outputs = process_task_flow_statements(statements, vec![state.clone()])?;
                ensure_no_new_task_bindings(&state, &block_outputs)?;
                outputs.extend(block_outputs);
            }
            Ok(outputs)
        }
        Statement::OnBlock { body, .. } => process_task_flow_statements(&body.statements, states),
        Statement::PlaceIn { body, on_error, .. } => {
            let mut outputs = Vec::new();
            for state in states {
                let body_outputs =
                    process_task_flow_statements(&body.statements, vec![state.clone()])?;
                ensure_no_new_task_bindings(&state, &body_outputs)?;
                outputs.extend(body_outputs);
                if let Some(on_error) = on_error {
                    let error_outputs =
                        process_task_flow_statements(&on_error.statements, vec![state.clone()])?;
                    ensure_no_new_task_bindings(&state, &error_outputs)?;
                    outputs.extend(error_outputs);
                }
            }
            Ok(outputs)
        }
        Statement::MemoryDecl { on_error, .. } => {
            if let Some(on_error) = on_error {
                let mut outputs = Vec::new();
                for state in states {
                    outputs.push(state.clone());
                    let error_outputs =
                        process_task_flow_statements(&on_error.statements, vec![state.clone()])?;
                    ensure_no_new_task_bindings(&state, &error_outputs)?;
                    outputs.extend(error_outputs);
                }
                Ok(outputs)
            } else {
                Ok(states)
            }
        }
        Statement::DangerAssignOnError { args, on_error, .. }
        | Statement::DangerCallOnError { args, on_error, .. } => {
            let mut outputs = Vec::new();
            for mut state in states {
                apply_task_flow_expressions(stmt, args, &mut state)?;
                outputs.push(state.clone());
                let error_outputs =
                    process_task_flow_statements(&on_error.statements, vec![state.clone()])?;
                ensure_no_new_task_bindings(&state, &error_outputs)?;
                outputs.extend(error_outputs);
            }
            Ok(outputs)
        }
        Statement::ListPopOnError { on_error, .. } => {
            let mut outputs = states.clone();
            for state in states {
                let error_outputs =
                    process_task_flow_statements(&on_error.statements, vec![state.clone()])?;
                ensure_no_new_task_bindings(&state, &error_outputs)?;
                outputs.extend(error_outputs);
            }
            Ok(outputs)
        }
        Statement::BreakStatement { .. } | Statement::ContinueStatement { .. } => {
            for state in &states {
                if let Some((name, binding)) = state.iter().next() {
                    return Err(open_task_error(name, binding));
                }
            }
            Ok(Vec::new())
        }
        Statement::FunctionDef { .. }
        | Statement::StructDecl { .. }
        | Statement::MemoryClear { .. }
        | Statement::IncDec { .. }
        | Statement::PassStatement { .. }
        | Statement::LabelDecl { .. } => Ok(states),
    }
}

fn process_task_flow_statements(
    statements: &[Statement],
    mut states: Vec<TaskFlowState>,
) -> Result<Vec<TaskFlowState>, String> {
    for stmt in statements {
        if states.is_empty() {
            break;
        }
        states = process_task_flow_statement(stmt, states)?;
    }
    Ok(states)
}

fn ensure_task_flow_finished(states: &[TaskFlowState]) -> Result<(), String> {
    for state in states {
        if let Some((name, binding)) = state.iter().next() {
            return Err(open_task_error(name, binding));
        }
    }
    Ok(())
}

fn validate_task_lifecycle(program: &Program) -> Result<(), String> {
    let top_level =
        process_task_flow_statements(&program.statements, vec![TaskFlowState::default()])?;
    ensure_task_flow_finished(&top_level)?;

    for stmt in &program.statements {
        match stmt {
            Statement::FunctionDef { body, .. } => {
                let states =
                    process_task_flow_statements(&body.statements, vec![TaskFlowState::default()])?;
                ensure_task_flow_finished(&states)?;
            }
            Statement::StructDecl { methods, .. } => {
                for method in methods {
                    let states = process_task_flow_statements(
                        &method.body.statements,
                        vec![TaskFlowState::default()],
                    )?;
                    ensure_task_flow_finished(&states)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn infer_expression_type(
    expr: &Expression,
    scope: &HashMap<String, ValueType>,
    memory_state: &MemoryState,
    functions: &HashMap<String, FunctionSig>,
    structs: &HashMap<String, StructInfo>,
    fn_ctx: Option<&FnContext>,
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
            let mut elem_ty =
                infer_expression_type(&items[0], scope, memory_state, functions, structs, fn_ctx)?;
            for item in &items[1..] {
                let item_ty =
                    infer_expression_type(item, scope, memory_state, functions, structs, fn_ctx)?;
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
        Expression::VariableReference(name) => {
            if let Some(ty) = builtin_constant_type(name) {
                Ok(ty)
            } else {
                if let Some(memory_name) = memory_state.variable_memory.get(name)
                    && memory_state
                        .memories
                        .get(memory_name)
                        .map(|binding| binding.is_cleared)
                        .unwrap_or(false)
                {
                    return Err(sem_err(
                        SEM_MEMORY_LIFETIME,
                        format!(
                            "use-after-clear: '{}' refers to memory '{}' that was cleared.",
                            name, memory_name
                        ),
                    ));
                }
                scope.get(name).cloned().ok_or_else(|| {
                    sem_err(
                        SEM_USE_BEFORE_DEF,
                        format!(
                            "use-before-definition: '{}' is not defined in current scope.",
                            name
                        ),
                    )
                })
            }
        }
        Expression::MemberAccess { base, .. } => {
            let owner_ty = if base == "my" {
                if let Some(ctx) = fn_ctx {
                    if let Some(self_name) = ctx.self_struct.as_ref() {
                        ValueType::Struct(self_name.clone())
                    } else {
                        return Err(sem_err(
                            SEM_INVALID_CONTEXT,
                            "my is only allowed inside struct methods.".to_string(),
                        ));
                    }
                } else {
                    return Err(sem_err(
                        SEM_INVALID_CONTEXT,
                        "my is only allowed inside struct methods.".to_string(),
                    ));
                }
            } else {
                if let Some(memory_name) = memory_state.variable_memory.get(base)
                    && memory_state
                        .memories
                        .get(memory_name)
                        .map(|binding| binding.is_cleared)
                        .unwrap_or(false)
                {
                    return Err(sem_err(
                        SEM_MEMORY_LIFETIME,
                        format!(
                            "use-after-clear: '{}' refers to memory '{}' that was cleared.",
                            base, memory_name
                        ),
                    ));
                }
                scope.get(base).cloned().ok_or_else(|| {
                    sem_err(
                        SEM_USE_BEFORE_DEF,
                        format!(
                            "use-before-definition: '{}' is not defined in current scope.",
                            base
                        ),
                    )
                })?
            };
            if let Expression::MemberAccess { field, .. } = expr {
                let ValueType::Struct(owner) = owner_ty else {
                    return Err(sem_err(
                        SEM_TYPE_MISMATCH,
                        format!(
                            "member access requires struct receiver, got {:?}.",
                            owner_ty
                        ),
                    ));
                };
                let Some(info) = structs.get(&owner) else {
                    return Err(sem_err(
                        SEM_TYPE_MISMATCH,
                        format!("unknown struct type '{}'.", owner),
                    ));
                };
                let Some(ft) = info.fields.get(field) else {
                    return Err(sem_err(
                        SEM_TYPE_MISMATCH,
                        format!("unknown field '{}.{}'.", owner, field),
                    ));
                };
                Ok(ft.clone())
            } else {
                Ok(ValueType::Unknown)
            }
        }
        Expression::Index { base, index } => {
            let base_ty =
                infer_expression_type(base, scope, memory_state, functions, structs, fn_ctx)?;
            let idx_ty =
                infer_expression_type(index, scope, memory_state, functions, structs, fn_ctx)?;
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
                    format!(
                        "index access is supported only for List/Text, got {:?}.",
                        other
                    ),
                )),
            }
        }
        Expression::Call { name, args } => {
            if name == "channel" {
                if args.len() != 1 {
                    return Err(sem_err(
                        SEM_CHANNEL_RULE,
                        format!(
                            "channel(N) expects one capacity argument, got {}.",
                            args.len()
                        ),
                    ));
                }
                let capacity_ty = infer_expression_type(
                    &args[0],
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx,
                )?;
                if capacity_ty != ValueType::Int {
                    return Err(sem_err(
                        SEM_CHANNEL_RULE,
                        format!("channel(N) expects Int capacity, got {:?}.", capacity_ty),
                    ));
                }
                return Ok(ValueType::Channel(Box::new(ValueType::Unknown)));
            }
            if let Some((base, method)) = name.split_once('.') {
                if (method == "send" || method == "receive")
                    && !memory_state.channels.contains_key(base)
                {
                    let receiver_ty = scope.get(base).cloned().unwrap_or(ValueType::Unknown);
                    if !matches!(receiver_ty, ValueType::Unknown) {
                        return Err(sem_err(
                            SEM_CHANNEL_RULE,
                            format!(
                                "channel operation '{}.{}' expects Channel receiver, got {:?}.",
                                base, method, receiver_ty
                            ),
                        ));
                    }
                }
                if let Some(channel_elem_ty) = memory_state.channels.get(base).cloned() {
                    return match method {
                        "send" => {
                            if args.len() != 1 {
                                return Err(sem_err(
                                    SEM_CHANNEL_RULE,
                                    format!(
                                        "send expects one message argument, got {}.",
                                        args.len()
                                    ),
                                ));
                            }
                            let actual_ty = infer_expression_type(
                                &args[0],
                                scope,
                                memory_state,
                                functions,
                                structs,
                                fn_ctx,
                            )?;
                            if !can_assign(&channel_elem_ty, &actual_ty) {
                                return Err(sem_err(
                                    SEM_CHANNEL_RULE,
                                    format!(
                                        "channel send type mismatch: expected {:?}, got {:?}.",
                                        channel_elem_ty, actual_ty
                                    ),
                                ));
                            }
                            if let Some(source_memory) = infer_expression_memory_provenance(
                                &args[0],
                                &actual_ty,
                                scope,
                                memory_state,
                                functions,
                                structs,
                                fn_ctx,
                            )? && !memory_is_external(memory_state, &source_memory)
                            {
                                return Err(sem_err(
                                    SEM_CHANNEL_RULE,
                                    format!(
                                        "channel messages must be value-safe: cannot send region-owned value from local Memory '{}'.",
                                        source_memory
                                    ),
                                ));
                            }
                            Ok(ValueType::Int)
                        }
                        "receive" => {
                            if !args.is_empty() {
                                return Err(sem_err(
                                    SEM_CHANNEL_RULE,
                                    format!("receive expects no arguments, got {}.", args.len()),
                                ));
                            }
                            Ok(channel_elem_ty)
                        }
                        _ => Err(sem_err(
                            SEM_CHANNEL_RULE,
                            format!("unsupported Channel operation '{}.{}'.", base, method),
                        )),
                    };
                }
            }
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
                        let arg_ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        match arg_ty {
                            ValueType::List(_) | ValueType::Text => Ok(ValueType::Int),
                            _ => Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'len' expects List or Text, got {:?}.", arg_ty),
                            )),
                        }
                    }
                    Builtin::Contains => {
                        let hay_ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let needle_ty = infer_expression_type(
                            &args[1],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
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
                        let hay_ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let needle_ty = infer_expression_type(
                            &args[1],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
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
                        let text_ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let start_ty = infer_expression_type(
                            &args[1],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let end_ty = infer_expression_type(
                            &args[2],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if text_ty != ValueType::Text
                            || start_ty != ValueType::Int
                            || end_ty != ValueType::Int
                        {
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
                        let a = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let b = infer_expression_type(
                            &args[1],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if a != ValueType::Text || b != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin 'concat' expects (Text, Text), got ({:?}, {:?}).",
                                    a, b
                                ),
                            ));
                        }
                        Ok(ValueType::Text)
                    }
                    Builtin::FsList => {
                        let path_ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if path_ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'fs.list' expects (Text), got ({:?}).", path_ty),
                            ));
                        }
                        Ok(ValueType::List(Box::new(ValueType::Text)))
                    }
                    Builtin::FsIsDir => {
                        let path_ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if path_ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'fs.is_dir' expects (Text), got ({:?}).", path_ty),
                            ));
                        }
                        Ok(ValueType::Bool)
                    }
                    Builtin::FsJoin => {
                        let a = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let b = infer_expression_type(
                            &args[1],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if a != ValueType::Text || b != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin 'fs.join' expects (Text, Text), got ({:?}, {:?}).",
                                    a, b
                                ),
                            ));
                        }
                        Ok(ValueType::Text)
                    }
                    Builtin::Args => Ok(ValueType::List(Box::new(ValueType::Text))),
                    Builtin::Output => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        match ty {
                            ValueType::Int
                            | ValueType::Float
                            | ValueType::Bool
                            | ValueType::Char
                            | ValueType::Text => Ok(ValueType::Int),
                            _ => Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'output' unsupported argument type: {:?}.", ty),
                            )),
                        }
                    }
                    Builtin::Input => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'input' expects (Text), got ({:?}).", ty),
                            ));
                        }
                        Ok(ValueType::Text)
                    }
                    Builtin::Read => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if ty != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'read' expects (Text), got ({:?}).", ty),
                            ));
                        }
                        Ok(ValueType::Text)
                    }
                    Builtin::Write => {
                        let p = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let d = infer_expression_type(
                            &args[1],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if p != ValueType::Text || d != ValueType::Text {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin 'write' expects (Text, Text), got ({:?}, {:?}).",
                                    p, d
                                ),
                            ));
                        }
                        Ok(ValueType::Int)
                    }
                    Builtin::Abs => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        ensure_numeric_args(name, std::slice::from_ref(&ty))?;
                        Ok(numeric_result_type(&[ty], true))
                    }
                    Builtin::Min | Builtin::Max => {
                        let a = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let b = infer_expression_type(
                            &args[1],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        ensure_numeric_args(name, &[a.clone(), b.clone()])?;
                        Ok(numeric_result_type(&[a, b], true))
                    }
                    Builtin::Clamp => {
                        let x = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let lo = infer_expression_type(
                            &args[1],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let hi = infer_expression_type(
                            &args[2],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        ensure_numeric_args(name, &[x.clone(), lo.clone(), hi.clone()])?;
                        Ok(numeric_result_type(&[x, lo, hi], true))
                    }
                    Builtin::Floor
                    | Builtin::Ceil
                    | Builtin::Round
                    | Builtin::Sin
                    | Builtin::Cos
                    | Builtin::Sqrt
                    | Builtin::DegToRad
                    | Builtin::RadToDeg => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if !is_numeric_type(&ty) {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin '{}' expects numeric argument, got {:?}.",
                                    name, ty
                                ),
                            ));
                        }
                        Ok(ValueType::Float)
                    }
                    Builtin::Atan2 | Builtin::Root => {
                        let a = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        let b = infer_expression_type(
                            &args[1],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if !is_numeric_type(&a) || !is_numeric_type(&b) {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin '{}' expects numeric arguments, got ({:?}, {:?}).",
                                    name, a, b
                                ),
                            ));
                        }
                        Ok(ValueType::Float)
                    }
                };
            }
            if let Some((base, method)) = name.split_once('.') {
                let owner_ty = if base == "my" {
                    if let Some(ctx) = fn_ctx {
                        if let Some(self_name) = ctx.self_struct.as_ref() {
                            ValueType::Struct(self_name.clone())
                        } else {
                            return Err(sem_err(
                                SEM_INVALID_CONTEXT,
                                "my is only allowed inside struct methods.".to_string(),
                            ));
                        }
                    } else {
                        return Err(sem_err(
                            SEM_INVALID_CONTEXT,
                            "my is only allowed inside struct methods.".to_string(),
                        ));
                    }
                } else {
                    if let Some(memory_name) = memory_state.variable_memory.get(base)
                        && memory_state
                            .memories
                            .get(memory_name)
                            .map(|binding| binding.is_cleared)
                            .unwrap_or(false)
                    {
                        return Err(sem_err(
                            SEM_MEMORY_LIFETIME,
                            format!(
                                "use-after-clear: '{}' refers to memory '{}' that was cleared.",
                                base, memory_name
                            ),
                        ));
                    }
                    scope.get(base).cloned().ok_or_else(|| {
                        sem_err(
                            SEM_USE_BEFORE_DEF,
                            format!(
                                "use-before-definition: '{}' is not defined in current scope.",
                                base
                            ),
                        )
                    })?
                };
                let ValueType::Struct(owner) = owner_ty else {
                    return Err(sem_err(
                        SEM_TYPE_MISMATCH,
                        format!("method call requires struct receiver, got {:?}.", owner_ty),
                    ));
                };
                let Some(info) = structs.get(&owner) else {
                    return Err(sem_err(
                        SEM_TYPE_MISMATCH,
                        format!("unknown struct type '{}'.", owner),
                    ));
                };
                let Some(sig) = info.methods.get(method) else {
                    return Err(sem_err(
                        SEM_UNKNOWN_FUNCTION,
                        format!("unknown method '{}.{}'.", owner, method),
                    ));
                };
                if args.len() != sig.param_types.len() {
                    return Err(sem_err(
                        SEM_ARG_COUNT,
                        format!(
                            "argument count mismatch for '{}.{}': expected {}, got {}.",
                            owner,
                            method,
                            sig.param_types.len(),
                            args.len()
                        ),
                    ));
                }
                for (arg, expected_ty) in args.iter().zip(sig.param_types.iter()) {
                    let actual_ty = infer_expression_type(
                        arg,
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx,
                    )?;
                    if !can_assign(expected_ty, &actual_ty) {
                        return Err(sem_err(
                            SEM_ARG_TYPE,
                            format!(
                                "argument type mismatch for '{}.{}': expected {:?}, got {:?}.",
                                owner, method, expected_ty, actual_ty
                            ),
                        ));
                    }
                }
                return Ok(sig.return_type.clone().unwrap_or(ValueType::Unknown));
            }
            let Some(sig) = functions.get(name) else {
                return Err(sem_err(
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in expression call.", name),
                ));
            };
            validate_call_args(
                name,
                args,
                sig,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx,
            )?;
            Ok(sig.return_type.clone().unwrap_or(ValueType::Unknown))
        }
        Expression::RunTask { call_name, args } => {
            let Some(sig) = functions.get(call_name) else {
                return Err(sem_err(
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in run task expression.", call_name),
                ));
            };
            if sig.is_danger {
                return Err(sem_err(
                    SEM_TASK_RULE,
                    format!(
                        "danger fn '{}' cannot be used as task entry in the v1.2 runtime MVP.",
                        call_name
                    ),
                ));
            }
            validate_call_args(
                call_name,
                args,
                sig,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx,
            )?;
            for (index, (arg, param_ty)) in args.iter().zip(&sig.param_types).enumerate() {
                let actual_ty =
                    infer_expression_type(arg, scope, memory_state, functions, structs, fn_ctx)?;
                if !is_task_safe_boundary_type(param_ty, structs, true)
                    || !is_task_safe_boundary_type(&actual_ty, structs, true)
                {
                    return Err(sem_err(
                        SEM_TASK_RULE,
                        format!(
                            "task-unsafe argument {} in run '{}': type {:?} cannot cross a task boundary.",
                            index + 1,
                            call_name,
                            actual_ty
                        ),
                    ));
                }
                if let Some(memory_name) = infer_expression_memory_provenance(
                    arg,
                    &actual_ty,
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx,
                )? {
                    return Err(sem_err(
                        SEM_TASK_RULE,
                        format!(
                            "task-unsafe argument {} in run '{}': region-owned value from Memory '{}' cannot cross a task boundary.",
                            index + 1,
                            call_name,
                            memory_name
                        ),
                    ));
                }
            }
            if let Some(result_ty) = sig.return_type.as_ref()
                && sig.has_explicit_return
                && !is_task_safe_boundary_type(result_ty, structs, false)
            {
                return Err(sem_err(
                    SEM_TASK_RULE,
                    format!(
                        "task-unsafe result from '{}': type {:?} cannot cross a task boundary.",
                        call_name, result_ty
                    ),
                ));
            }
            let result_type = if sig.has_explicit_return {
                sig.return_type.clone().map(Box::new)
            } else {
                None
            };
            Ok(ValueType::Task(result_type))
        }
        Expression::WaitTask { task_name } => {
            let Some(task_ty) = scope.get(task_name).cloned() else {
                return Err(sem_err(
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined in current scope.",
                        task_name
                    ),
                ));
            };
            if !matches!(task_ty, ValueType::Task(_)) {
                return Err(sem_err(
                    SEM_TASK_RULE,
                    format!("wait expects Task handle, got {:?}.", task_ty),
                ));
            }
            let Some(binding) = memory_state.tasks.get(task_name) else {
                return Err(sem_err(
                    SEM_TASK_RULE,
                    format!(
                        "task handle '{}' is not available in current scope.",
                        task_name
                    ),
                ));
            };
            if binding.waited {
                return Err(sem_err(
                    SEM_TASK_RULE,
                    format!("task handle '{}' was already waited.", task_name),
                ));
            }
            Ok(binding.result_type.clone().unwrap_or(ValueType::Unknown))
        }
        Expression::Stopping => {
            if fn_ctx.map(|ctx| ctx.is_task_context) != Some(true) {
                return Err(sem_err(
                    SEM_TASK_RULE,
                    "stopping is only available inside a function launched with run.".to_string(),
                ));
            }
            Ok(ValueType::Bool)
        }
        Expression::BinaryOp { op, left, right } => {
            if op == "neg" {
                let lt =
                    infer_expression_type(left, scope, memory_state, functions, structs, fn_ctx)?;
                if lt == ValueType::Int || lt == ValueType::Float {
                    return Ok(lt);
                }
                return Err(sem_err(
                    SEM_TYPE_MISMATCH,
                    "unary '-' requires numeric operand.".to_string(),
                ));
            }
            if op == "not" {
                let lt =
                    infer_expression_type(left, scope, memory_state, functions, structs, fn_ctx)?;
                if lt == ValueType::Bool || lt == ValueType::Int {
                    return Ok(ValueType::Bool);
                }
                return Err(sem_err(
                    SEM_TYPE_MISMATCH,
                    "unary 'not' requires bool/int operand.".to_string(),
                ));
            }
            let lt = infer_expression_type(left, scope, memory_state, functions, structs, fn_ctx)?;
            let rt = if let Some(r) = right {
                infer_expression_type(r, scope, memory_state, functions, structs, fn_ctx)?
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
                let _ =
                    infer_expression_type(value, scope, memory_state, functions, structs, fn_ctx)?;
            }
            Ok(ValueType::Unknown)
        }
    }
}

fn infer_expression_memory_provenance(
    expr: &Expression,
    result_ty: &ValueType,
    scope: &HashMap<String, ValueType>,
    memory_state: &MemoryState,
    functions: &HashMap<String, FunctionSig>,
    structs: &HashMap<String, StructInfo>,
    fn_ctx: Option<&FnContext>,
) -> Result<Option<String>, String> {
    if !region_relevant(result_ty, structs) {
        return Ok(None);
    }

    match expr {
        Expression::VariableReference(name) => Ok(memory_state.variable_memory.get(name).cloned()),
        Expression::MemberAccess { base, .. } => {
            Ok(memory_state.variable_memory.get(base).cloned())
        }
        Expression::Index { base, .. } => {
            let base_ty =
                infer_expression_type(base, scope, memory_state, functions, structs, fn_ctx)?;
            if region_relevant(&base_ty, structs) {
                infer_expression_memory_provenance(
                    base,
                    &base_ty,
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx,
                )
            } else {
                Ok(None)
            }
        }
        Expression::Call { name, .. } => {
            if let Some((base, _)) = name.split_once('.')
                && let Some(memory_name) = memory_state.variable_memory.get(base)
            {
                return Ok(Some(memory_name.clone()));
            }
            Ok(memory_state.active_memory.clone())
        }
        Expression::WaitTask { task_name } => {
            Ok(memory_state.variable_memory.get(task_name).cloned())
        }
        Expression::RunTask { .. } | Expression::Stopping => Ok(None),
        Expression::LiteralString(_)
        | Expression::ListLiteral(_)
        | Expression::StructConstruction { .. } => Ok(memory_state.active_memory.clone()),
        Expression::BinaryOp { .. } => Ok(None),
        Expression::LiteralInt(_) | Expression::LiteralFloat(_) | Expression::LiteralBool(_) => {
            Ok(None)
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
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => {
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
        Expression::MemberAccess { base, .. } => base == name,
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
        Expression::Call { args, .. } | Expression::RunTask { args, .. } => {
            args.iter().any(|a| contains_variable(a, name))
        }
        Expression::WaitTask { task_name } => task_name == name,
        Expression::Index { base, index } => {
            contains_variable(base, name) || contains_variable(index, name)
        }
        Expression::ListLiteral(items) => items.iter().any(|a| contains_variable(a, name)),
        Expression::LiteralString(_) => false,
        _ => false,
    }
}
