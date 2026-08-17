use std::collections::{HashMap, HashSet};

use crate::ast_nodes::{
    BlockStatement, BorrowMode, Expression, ForLoopStyle, FunctionParam, LabelVariant, Program,
    Statement,
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
    Time,
    Duration,
    ByteSize,
    Angle,
    Vec2,
    Vec3,
    Vec4,
    Color,
    Rect,
    Canvas,
    Window,
    Memory,
    Interrupt,
    Task(Option<Box<ValueType>>),
    Channel(Box<ValueType>),
    List(Box<ValueType>),
    Struct(String),
    Label(String),
    Tag(String),
    Unknown,
}

#[derive(Clone, Debug)]
struct FunctionSig {
    is_danger: bool,
    return_type: Option<ValueType>,
    has_explicit_return: bool,
    param_types: Vec<ValueType>,
    param_borrows: Vec<BorrowMode>,
}

#[derive(Clone)]
struct FnContext {
    is_danger: bool,
    return_type: Option<ValueType>,
    self_struct: Option<String>,
    is_task_context: bool,
    is_timed_error_context: bool,
}

#[derive(Clone, Debug, Default)]
struct MemoryBinding {
    is_external: bool,
    is_cleared: bool,
    parent: Option<String>,
}

#[derive(Clone, Debug)]
struct TaskBinding {
    result_type: Option<ValueType>,
    waited: bool,
    stopped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResourceLifecycle {
    Open,
    MaybeClosed,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnershipState {
    Owned,
    MaybeMoved,
    Moved,
}

impl OwnershipState {
    fn merge(self, other: Self) -> Self {
        if self == other {
            self
        } else {
            Self::MaybeMoved
        }
    }
}

#[derive(Clone, Debug)]
struct OwnershipBinding {
    state: OwnershipState,
    moved_to: Option<String>,
}

impl ResourceLifecycle {
    fn merge(self, other: Self) -> Self {
        if self == other {
            self
        } else {
            Self::MaybeClosed
        }
    }
}

#[derive(Clone, Debug, Default)]
struct MemoryState {
    active_memory: Option<String>,
    memories: HashMap<String, MemoryBinding>,
    variable_memory: HashMap<String, String>,
    tasks: HashMap<String, TaskBinding>,
    channels: HashMap<String, ValueType>,
    channel_owners: HashSet<String>,
    resource_lifecycles: HashMap<String, ResourceLifecycle>,
    ownership: HashMap<String, OwnershipBinding>,
    interrupt_owners: HashSet<String>,
    constants: HashSet<String>,
    views: HashSet<String>,
    labels: HashMap<String, HashSet<String>>,
    tags: HashMap<String, HashSet<String>>,
}

fn is_movable_resource(ty: &ValueType) -> bool {
    matches!(
        ty,
        ValueType::Canvas | ValueType::Window | ValueType::Interrupt | ValueType::Channel(_)
    )
}

fn requires_explicit_resource_mode(ty: &ValueType) -> bool {
    matches!(
        ty,
        ValueType::Canvas | ValueType::Window | ValueType::Interrupt
    )
}

fn merge_ownership_states(
    target: &mut MemoryState,
    branches: &[&MemoryState],
    original: &MemoryState,
) {
    for (name, original_binding) in &original.ownership {
        let mut branch_bindings = branches
            .iter()
            .map(|branch| branch.ownership.get(name).unwrap_or(original_binding));
        let first = branch_bindings.next().unwrap_or(original_binding);
        let merged = branch_bindings.fold(first.state, |state, binding| state.merge(binding.state));
        let moved_to = branches
            .iter()
            .filter_map(|branch| branch.ownership.get(name))
            .find_map(|binding| binding.moved_to.clone())
            .or_else(|| original_binding.moved_to.clone());
        target.ownership.insert(
            name.clone(),
            OwnershipBinding {
                state: merged,
                moved_to,
            },
        );
    }
}

fn require_owned_resource(state: &MemoryState, name: &str) -> Result<(), String> {
    match state.ownership.get(name) {
        Some(OwnershipBinding {
            state: OwnershipState::Moved,
            moved_to,
        }) => Err(sem_err(
            SEM_INVALID_CONTEXT,
            format!(
                "resource '{}' was moved{} and is no longer available.",
                name,
                moved_to
                    .as_deref()
                    .map(|target| format!(" to '{target}'"))
                    .unwrap_or_default()
            ),
        )),
        Some(OwnershipBinding {
            state: OwnershipState::MaybeMoved,
            ..
        }) => Err(sem_err(
            SEM_INVALID_CONTEXT,
            format!(
                "resource '{}' is available only on some control-flow paths after a move; move it after the branch or keep all uses inside each branch.",
                name
            ),
        )),
        _ => Ok(()),
    }
}

fn mark_resource_moved(
    state: &mut MemoryState,
    name: &str,
    destination: impl Into<String>,
) -> Result<(), String> {
    require_owned_resource(state, name)?;
    let Some(binding) = state.ownership.get_mut(name) else {
        return Err(sem_err(
            SEM_INVALID_CONTEXT,
            format!("'move {}' requires an owning resource binding.", name),
        ));
    };
    binding.state = OwnershipState::Moved;
    binding.moved_to = Some(destination.into());
    Ok(())
}

fn ensure_loop_preserves_ownership(
    stmt: &Statement,
    original: &MemoryState,
    body: &MemoryState,
) -> Result<(), String> {
    for (name, original_binding) in &original.ownership {
        let body_state = body
            .ownership
            .get(name)
            .map(|binding| binding.state)
            .unwrap_or(original_binding.state);
        if body_state != original_binding.state {
            return Err(err_at_code(
                stmt,
                SEM_INVALID_CONTEXT,
                format!(
                    "resource '{}' cannot be moved from a repeating loop body; create the owner inside the iteration, return immediately, or move it after the loop.",
                    name
                ),
            ));
        }
    }
    Ok(())
}

fn merge_resource_lifecycles(
    target: &mut MemoryState,
    branches: &[&MemoryState],
    original: &MemoryState,
) {
    for (name, original_state) in &original.resource_lifecycles {
        let mut branch_states = branches.iter().map(|branch| {
            branch
                .resource_lifecycles
                .get(name)
                .copied()
                .unwrap_or(*original_state)
        });
        let first = branch_states.next().unwrap_or(*original_state);
        let merged = branch_states.fold(first, ResourceLifecycle::merge);
        target.resource_lifecycles.insert(name.clone(), merged);
    }
    merge_ownership_states(target, branches, original);
}

fn require_open_resource(state: &MemoryState, name: &str, operation: &str) -> Result<(), String> {
    require_owned_resource(state, name)?;
    match state.resource_lifecycles.get(name) {
        Some(ResourceLifecycle::MaybeClosed) => Err(sem_err(
            SEM_INVALID_CONTEXT,
            format!(
                "resource '{}' may be closed before '{}'; handle the operation with 'on error' or restructure the control flow.",
                name, operation
            ),
        )),
        Some(ResourceLifecycle::Closed) => Err(sem_err(
            SEM_INVALID_CONTEXT,
            format!(
                "resource '{}' is already closed before '{}'; handle the operation with 'on error'.",
                name, operation
            ),
        )),
        _ => Ok(()),
    }
}

fn read_only_binding_kind(state: &MemoryState, name: &str) -> Option<&'static str> {
    if state.views.contains(name) {
        Some("view parameter")
    } else if state.constants.contains(name) {
        Some("constant binding")
    } else {
        None
    }
}

#[derive(Clone, Debug)]
struct StructInfo {
    fields: HashMap<String, ValueType>,
    hidden_fields: std::collections::HashSet<String>,
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
        | Statement::TagDecl { loc, .. }
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
    let mut labels: HashMap<String, Vec<LabelVariant>> = HashMap::new();
    let mut tags: HashMap<String, Vec<String>> = HashMap::new();
    let mut structs: HashMap<String, StructInfo> = HashMap::new();
    let task_context_functions = collect_task_context_functions(&program.statements);

    for stmt in &program.statements {
        if let Statement::FunctionDef {
            name,
            is_danger,
            returns,
            params,
            uses_returns_keyword: _,
            is_local,
            ..
        } = stmt
        {
            if let Some(return_name) = returns.as_deref() {
                let return_ty = parse_type_name(return_name);
                ensure_memory_type_allowed(stmt, &return_ty, "function return type", false)?;
                ensure_task_type_allowed(stmt, &return_ty, "function return type", false)?;
                ensure_channel_type_allowed(
                    stmt,
                    &return_ty,
                    "function return type",
                    matches!(return_ty, ValueType::Channel(_)),
                )?;
            }
            for param in params {
                if let Some(param_name) = param.param_type.as_deref() {
                    let param_ty = parse_type_name(param_name);
                    if requires_explicit_resource_mode(&param_ty)
                        && param.borrow == BorrowMode::Value
                    {
                        return Err(err_at_code(
                            stmt,
                            SEM_INVALID_CONTEXT,
                            format!(
                                "resource parameter '{}' must use 'direct', 'view', or 'move'.",
                                param.name
                            ),
                        ));
                    }
                    if param.borrow == BorrowMode::Move && !is_movable_resource(&param_ty) {
                        return Err(err_at_code(
                            stmt,
                            SEM_INVALID_CONTEXT,
                            format!(
                                "'move' parameter '{}' requires an owning resource type, got {:?}.",
                                param.name, param_ty
                            ),
                        ));
                    }
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
            if !*is_local
                && (functions.contains_key(name)
                    || labels.contains_key(name)
                    || structs.contains_key(name))
            {
                return Err(err_at_code(
                    stmt,
                    SEM_REDECLARATION,
                    format!(
                        "top-level symbol collision for '{}'. use distinct names or qualification.",
                        name
                    ),
                ));
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
                    param_borrows: params.iter().map(|param| param.borrow).collect(),
                },
            );
        }
        if let Statement::LabelDecl {
            name,
            variants,
            is_local,
            ..
        } = stmt
        {
            if !*is_local
                && (functions.contains_key(name)
                    || labels.contains_key(name)
                    || tags.contains_key(name)
                    || structs.contains_key(name))
            {
                return Err(err_at_code(
                    stmt,
                    SEM_REDECLARATION,
                    format!(
                        "top-level symbol collision for '{}'. use distinct names or qualification.",
                        name
                    ),
                ));
            }
            labels.insert(name.clone(), variants.clone());
        }
        if let Statement::TagDecl {
            name,
            variants,
            is_local,
            ..
        } = stmt
        {
            if !*is_local
                && (functions.contains_key(name)
                    || labels.contains_key(name)
                    || tags.contains_key(name)
                    || structs.contains_key(name))
            {
                return Err(err_at_code(
                    stmt,
                    SEM_REDECLARATION,
                    format!(
                        "top-level symbol collision for '{}'. use distinct names or qualification.",
                        name
                    ),
                ));
            }
            tags.insert(name.clone(), variants.clone());
        }
        if let Statement::StructDecl {
            name,
            fields,
            methods,
            is_local,
            ..
        } = stmt
        {
            if !*is_local
                && (functions.contains_key(name)
                    || labels.contains_key(name)
                    || tags.contains_key(name)
                    || structs.contains_key(name))
            {
                return Err(err_at_code(
                    stmt,
                    SEM_REDECLARATION,
                    format!(
                        "top-level symbol collision for '{}'. use distinct names or qualification.",
                        name
                    ),
                ));
            }
            let mut fmap = HashMap::new();
            let mut hidden = std::collections::HashSet::new();
            for f in fields {
                let field_ty = parse_type_name(&f.field_type);
                ensure_memory_type_allowed(stmt, &field_ty, "struct field type", false)?;
                ensure_task_type_allowed(stmt, &field_ty, "struct field type", false)?;
                ensure_channel_type_allowed(stmt, &field_ty, "struct field type", false)?;
                fmap.insert(f.name.clone(), field_ty);
                if f.is_hidden {
                    hidden.insert(f.name.clone());
                }
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
                        matches!(return_ty, ValueType::Channel(_)),
                    )?;
                }
                for param in &m.params {
                    if let Some(param_name) = param.param_type.as_deref() {
                        let param_ty = parse_type_name(param_name);
                        if requires_explicit_resource_mode(&param_ty)
                            && param.borrow == BorrowMode::Value
                        {
                            return Err(err_at_code(
                                stmt,
                                SEM_INVALID_CONTEXT,
                                format!(
                                    "resource parameter '{}' must use 'direct', 'view', or 'move'.",
                                    param.name
                                ),
                            ));
                        }
                        if param.borrow == BorrowMode::Move && !is_movable_resource(&param_ty) {
                            return Err(err_at_code(
                                stmt,
                                SEM_INVALID_CONTEXT,
                                format!(
                                    "'move' parameter '{}' requires an owning resource type, got {:?}.",
                                    param.name, param_ty
                                ),
                            ));
                        }
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
                        param_borrows: m.params.iter().map(|param| param.borrow).collect(),
                    },
                );
            }
            structs.insert(
                name.clone(),
                StructInfo {
                    fields: fmap,
                    hidden_fields: hidden,
                    methods: mmap,
                },
            );
        }
    }

    let nominal_types: HashMap<String, ValueType> = labels
        .keys()
        .map(|name| (name.clone(), ValueType::Label(name.clone())))
        .chain(
            tags.keys()
                .map(|name| (name.clone(), ValueType::Tag(name.clone()))),
        )
        .collect();

    // User-defined nominal types may be declared after functions. Resolve function
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
                .map(|name| parse_declared_type_name(name, &structs, &nominal_types))
                .or(Some(ValueType::Int));
            sig.param_types = params
                .iter()
                .map(|param| {
                    param
                        .param_type
                        .as_deref()
                        .map(|name| parse_declared_type_name(name, &structs, &nominal_types))
                        .unwrap_or(ValueType::Int)
                })
                .collect();
        }
    }

    validate_nominal_sets(&labels, &tags)?;
    let label_names: HashMap<String, Vec<String>> = labels
        .iter()
        .map(|(name, variants)| {
            (
                name.clone(),
                variants
                    .iter()
                    .map(|variant| variant.name.clone())
                    .collect(),
            )
        })
        .collect();

    let mut scope: HashMap<String, ValueType> = HashMap::new();
    let mut memory_state = MemoryState {
        labels: labels
            .iter()
            .map(|(name, variants)| {
                (
                    name.clone(),
                    variants
                        .iter()
                        .map(|variant| variant.name.clone())
                        .collect(),
                )
            })
            .collect(),
        tags: tags
            .iter()
            .map(|(name, variants)| (name.clone(), variants.iter().cloned().collect()))
            .collect(),
        ..MemoryState::default()
    };
    analyze_statements(
        &program.statements,
        &mut scope,
        &mut memory_state,
        &functions,
        &label_names,
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
                | "Time"
                | "Duration"
                | "ByteSize"
                | "Angle"
                | "List"
                | "Vec2"
                | "Vec3"
                | "Vec4"
                | "Color"
                | "Rect"
                | "Canvas"
                | "Window"
                | "Interrupt"
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
            Expression::WaitTask { .. } | Expression::Stopping | Expression::TimedOut => {}
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
                    uses_returns_keyword,
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
                    if returns.is_some() && !*uses_returns_keyword {
                        out.push(format!(
                            "style warning at line {}, col {}: prefer explicit 'returns <type>' in function declaration.",
                            loc.line, loc.column
                        ));
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

    fn merge_warning_states(
        target: &mut HashMap<String, ResourceLifecycle>,
        branches: &[&HashMap<String, ResourceLifecycle>],
        original: &HashMap<String, ResourceLifecycle>,
    ) {
        for (name, original_state) in original {
            let mut states = branches
                .iter()
                .map(|branch| branch.get(name).copied().unwrap_or(*original_state));
            let first = states.next().unwrap_or(*original_state);
            target.insert(name.clone(), states.fold(first, ResourceLifecycle::merge));
        }
    }

    fn visit_lifecycle_warnings(
        statements: &[Statement],
        states: &mut HashMap<String, ResourceLifecycle>,
        out: &mut Vec<String>,
    ) {
        for statement in statements {
            match statement {
                Statement::VarDecl {
                    name,
                    declared_type: Some(declared_type),
                    ..
                } if declared_type == "Window" || declared_type.starts_with("Channel(") => {
                    states.insert(name.clone(), ResourceLifecycle::Open);
                }
                Statement::ExpressionStatement { expr, .. } => {
                    if let Expression::Call { name, .. } = expr.as_ref()
                        && let Some((resource, "close")) = name.split_once('.')
                        && states.contains_key(resource)
                    {
                        states.insert(resource.to_string(), ResourceLifecycle::Closed);
                    }
                }
                Statement::DangerCallOnError {
                    call_name,
                    on_error,
                    loc,
                    ..
                } => {
                    if let Some((resource, operation)) = call_name.split_once('.')
                        && states.get(resource) == Some(&ResourceLifecycle::Closed)
                        && matches!(operation, "present" | "send" | "receive" | "close")
                    {
                        out.push(format!(
                            "style warning at line {}, col {}: resource '{}' is already closed; '{}.{}' is guaranteed to enter its 'on error' handler.",
                            loc.line, loc.column, resource, resource, operation
                        ));
                    }
                    if let Some((resource, "close")) = call_name.split_once('.')
                        && states.contains_key(resource)
                    {
                        states.insert(resource.to_string(), ResourceLifecycle::Closed);
                    }
                    let mut handler_states = states.clone();
                    visit_lifecycle_warnings(&on_error.statements, &mut handler_states, out);
                }
                Statement::DangerAssignOnError { on_error, .. }
                | Statement::ListPopOnError { on_error, .. } => {
                    let mut handler_states = states.clone();
                    visit_lifecycle_warnings(&on_error.statements, &mut handler_states, out);
                }
                Statement::IfStatement {
                    then_block,
                    else_block,
                    ..
                } => {
                    let original = states.clone();
                    let mut then_states = original.clone();
                    visit_lifecycle_warnings(&then_block.statements, &mut then_states, out);
                    let mut branches = Vec::new();
                    if !block_guarantees_termination(then_block) {
                        branches.push(then_states);
                    }
                    if let Some(else_block) = else_block {
                        let mut else_states = original.clone();
                        visit_lifecycle_warnings(&else_block.statements, &mut else_states, out);
                        if !block_guarantees_termination(else_block) {
                            branches.push(else_states);
                        }
                    } else {
                        branches.push(original.clone());
                    }
                    if !branches.is_empty() {
                        let refs = branches.iter().collect::<Vec<_>>();
                        merge_warning_states(states, &refs, &original);
                    }
                }
                Statement::WhenBlock {
                    cases, else_block, ..
                } => {
                    let original = states.clone();
                    let mut branches = Vec::new();
                    for (_, block) in cases {
                        let mut case_states = original.clone();
                        visit_lifecycle_warnings(&block.statements, &mut case_states, out);
                        if !block_guarantees_termination(block) {
                            branches.push(case_states);
                        }
                    }
                    if let Some(else_block) = else_block {
                        let mut else_states = original.clone();
                        visit_lifecycle_warnings(&else_block.statements, &mut else_states, out);
                        if !block_guarantees_termination(else_block) {
                            branches.push(else_states);
                        }
                    } else {
                        branches.push(original.clone());
                    }
                    if !branches.is_empty() {
                        let refs = branches.iter().collect::<Vec<_>>();
                        merge_warning_states(states, &refs, &original);
                    }
                }
                Statement::ForLoop { body, .. }
                | Statement::WhileLoop { body, .. }
                | Statement::LoopStatement { body, .. } => {
                    let original = states.clone();
                    let mut body_states = original.clone();
                    visit_lifecycle_warnings(&body.statements, &mut body_states, out);
                    merge_warning_states(states, &[&original, &body_states], &original);
                }
                Statement::FunctionDef { body, .. } => {
                    let mut function_states = HashMap::new();
                    visit_lifecycle_warnings(&body.statements, &mut function_states, out);
                }
                Statement::StructDecl { methods, .. } => {
                    for method in methods {
                        let mut method_states = HashMap::new();
                        visit_lifecycle_warnings(&method.body.statements, &mut method_states, out);
                    }
                }
                Statement::BlockStatement { statements, .. }
                | Statement::OnErrorBlock { statements, .. } => {
                    visit_lifecycle_warnings(statements, states, out);
                }
                Statement::PlaceIn { body, on_error, .. } => {
                    visit_lifecycle_warnings(&body.statements, states, out);
                    if let Some(on_error) = on_error {
                        let mut handler_states = states.clone();
                        visit_lifecycle_warnings(&on_error.statements, &mut handler_states, out);
                    }
                }
                Statement::MemoryDecl {
                    on_error: Some(on_error),
                    ..
                } => {
                    let mut handler_states = states.clone();
                    visit_lifecycle_warnings(&on_error.statements, &mut handler_states, out);
                }
                _ => {}
            }
        }
    }

    let mut warnings = Vec::new();
    visit_statements(&program.statements, &user_types, &mut warnings);
    visit_lifecycle_warnings(&program.statements, &mut HashMap::new(), &mut warnings);
    warnings
}

fn validate_nominal_sets(
    labels: &HashMap<String, Vec<LabelVariant>>,
    tags: &HashMap<String, Vec<String>>,
) -> Result<(), String> {
    for (name, variants) in labels {
        let mut variant_names = HashSet::new();
        let mut discriminants = HashSet::new();
        for variant in variants {
            if !variant_names.insert(&variant.name) {
                return Err(sem_err(
                    SEM_ERRORCODE_RULE,
                    format!(
                        "label '{}' contains duplicate variant '{}'.",
                        name, variant.name
                    ),
                ));
            }
            if !discriminants.insert(variant.discriminant) {
                return Err(sem_err(
                    SEM_ERRORCODE_RULE,
                    format!(
                        "label '{}' contains duplicate discriminant {}.",
                        name, variant.discriminant
                    ),
                ));
            }
        }
    }
    for (name, variants) in tags {
        let mut names = HashSet::new();
        for variant in variants {
            if !names.insert(variant) {
                return Err(sem_err(
                    SEM_ERRORCODE_RULE,
                    format!("tag '{}' contains duplicate variant '{}'.", name, variant),
                ));
            }
        }
    }
    if let Some(error_codes) = labels.get("ErrorCode") {
        if error_codes.is_empty() {
            return Err(sem_err(
                SEM_ERRORCODE_RULE,
                "label ErrorCode must define at least one variant.".to_string(),
            ));
        }
        if error_codes[0].name != "Ok" || error_codes[0].discriminant != 0 {
            return Err(sem_err(
                SEM_ERRORCODE_RULE,
                "label ErrorCode must start with 'Ok = 0' variant.".to_string(),
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
        "Interrupt" => ValueType::Interrupt,
        "Text" | "Path" => ValueType::Text,
        "Time" => ValueType::Time,
        "Duration" => ValueType::Duration,
        "ByteSize" => ValueType::ByteSize,
        "Angle" => ValueType::Angle,
        "Vec2" => ValueType::Vec2,
        "Vec3" => ValueType::Vec3,
        "Vec4" => ValueType::Vec4,
        "Color" => ValueType::Color,
        "Rect" => ValueType::Rect,
        "Canvas" => ValueType::Canvas,
        "Window" => ValueType::Window,
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

fn interrupt_safe_expression(expr: &Expression) -> bool {
    match expr {
        Expression::LiteralInt(_)
        | Expression::LiteralFloat(_)
        | Expression::LiteralBool(_)
        | Expression::LiteralChar(_)
        | Expression::LiteralDuration { .. }
        | Expression::LiteralByteSize { .. }
        | Expression::LiteralAngle { .. } => true,
        Expression::VariableReference(name) => {
            matches!(name.as_str(), "PI" | "TAU" | "E" | "EPSILON")
        }
        Expression::MemberAccess { .. } => true,
        Expression::BinaryOp { left, right, .. } => {
            interrupt_safe_expression(left)
                && right
                    .as_deref()
                    .map(interrupt_safe_expression)
                    .unwrap_or(true)
        }
        Expression::Call { name, args } => {
            matches!(
                name.as_str(),
                "abs"
                    | "min"
                    | "max"
                    | "clamp"
                    | "floor"
                    | "ceil"
                    | "round"
                    | "sin"
                    | "cos"
                    | "atan2"
                    | "sqrt"
                    | "root"
                    | "deg_to_rad"
                    | "rad_to_deg"
            ) && args.iter().all(interrupt_safe_expression)
        }
        Expression::LiteralString(_)
        | Expression::ListLiteral(_)
        | Expression::Index { .. }
        | Expression::DirectBorrow(_)
        | Expression::ViewBorrow(_)
        | Expression::Move(_)
        | Expression::RunTask { .. }
        | Expression::WaitTask { .. }
        | Expression::Stopping
        | Expression::TimedOut
        | Expression::StructConstruction { .. } => false,
    }
}

fn validate_interrupt_block(
    block: &BlockStatement,
    memory_state: &MemoryState,
) -> Result<(), String> {
    for statement in &block.statements {
        match statement {
            Statement::PassStatement { .. } => {}
            Statement::ExpressionStatement { expr, .. } => {
                let Expression::Call { name, args } = expr.as_ref() else {
                    return Err(sem_err(
                        SEM_INVALID_CONTEXT,
                        "interrupt handler permits only Channel.try_send statements and finite branching."
                            .to_string(),
                    ));
                };
                let Some((channel, method)) = name.split_once('.') else {
                    return Err(sem_err(
                        SEM_INVALID_CONTEXT,
                        format!(
                            "call '{}' is not interrupt-safe; use Channel.try_send as the normal-context bridge.",
                            name
                        ),
                    ));
                };
                if method != "try_send"
                    || !memory_state.channels.contains_key(channel)
                    || args.len() != 1
                    || !interrupt_safe_expression(&args[0])
                {
                    return Err(sem_err(
                        SEM_INVALID_CONTEXT,
                        format!(
                            "call '{}' is not interrupt-safe; only Channel.try_send with a non-allocating value expression is allowed.",
                            name
                        ),
                    ));
                }
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                if !interrupt_safe_expression(condition) {
                    return Err(sem_err(
                        SEM_INVALID_CONTEXT,
                        "interrupt condition must be a non-allocating scalar expression."
                            .to_string(),
                    ));
                }
                validate_interrupt_block(then_block, memory_state)?;
                if let Some(else_block) = else_block {
                    validate_interrupt_block(else_block, memory_state)?;
                }
            }
            _ => {
                return Err(sem_err(
                    SEM_INVALID_CONTEXT,
                    "operation is forbidden in interrupt context: handlers cannot allocate, block, perform I/O, manage tasks/resources, or call ordinary functions."
                        .to_string(),
                ));
            }
        }
    }
    Ok(())
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
            | Expression::DirectBorrow(_)
            | Expression::ViewBorrow(_)
            | Expression::Move(_)
            | Expression::MemberAccess { .. }
            | Expression::WaitTask { .. }
            | Expression::Stopping
            | Expression::TimedOut
            | Expression::LiteralInt(_)
            | Expression::LiteralFloat(_)
            | Expression::LiteralBool(_)
            | Expression::LiteralChar(_)
            | Expression::LiteralString(_)
            | Expression::LiteralDuration { .. }
            | Expression::LiteralByteSize { .. }
            | Expression::LiteralAngle { .. } => {}
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
                | Statement::LabelDecl { .. }
                | Statement::TagDecl { .. } => {}
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

fn is_color_constant(name: &str) -> bool {
    matches!(
        name,
        "black"
            | "red"
            | "green"
            | "yellow"
            | "blue"
            | "magenta"
            | "cyan"
            | "white"
            | "transparent"
            | "terminal_black"
            | "terminal_red"
            | "terminal_green"
            | "terminal_yellow"
            | "terminal_blue"
            | "terminal_magenta"
            | "terminal_cyan"
            | "terminal_white"
            | "terminal_bright_black"
            | "terminal_bright_red"
            | "terminal_bright_green"
            | "terminal_bright_yellow"
            | "terminal_bright_blue"
            | "terminal_bright_magenta"
            | "terminal_bright_cyan"
            | "terminal_bright_white"
    )
}

fn is_numeric_type(ty: &ValueType) -> bool {
    matches!(ty, ValueType::Int | ValueType::Float)
}

fn vector_dimension(ty: &ValueType) -> Option<usize> {
    match ty {
        ValueType::Vec2 => Some(2),
        ValueType::Vec3 => Some(3),
        ValueType::Vec4 => Some(4),
        _ => None,
    }
}

fn validate_expression_for_target(
    target: &ValueType,
    expr: &Expression,
    scope: &HashMap<String, ValueType>,
    memory_state: &MemoryState,
    functions: &HashMap<String, FunctionSig>,
    structs: &HashMap<String, StructInfo>,
    fn_ctx: Option<&FnContext>,
) -> Result<(), String> {
    if let Some(dimension) = vector_dimension(target) {
        if let Expression::StructConstruction { fields } = expr {
            let expected = ["x", "y", "z", "w"];
            let expected = &expected[..dimension];
            let mut actual: Vec<&str> = fields.keys().map(String::as_str).collect();
            actual.sort_unstable();
            let mut expected_sorted = expected.to_vec();
            expected_sorted.sort_unstable();
            if actual != expected_sorted {
                return Err(sem_err(
                    SEM_TYPE_MISMATCH,
                    format!(
                        "{:?} construction requires exactly fields {}, got {}.",
                        target,
                        expected.join(", "),
                        if actual.is_empty() {
                            "none".to_string()
                        } else {
                            actual.join(", ")
                        }
                    ),
                ));
            }
            for field in expected {
                let value_ty = infer_expression_type(
                    fields.get(*field).expect("validated vector field"),
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx,
                )?;
                if !is_numeric_type(&value_ty) {
                    return Err(sem_err(
                        SEM_TYPE_MISMATCH,
                        format!(
                            "{:?} field '{}' expects Int or Float, got {:?}.",
                            target, field, value_ty
                        ),
                    ));
                }
            }
        }
    } else if let (ValueType::List(element), Expression::ListLiteral(items)) = (target, expr) {
        for item in items {
            validate_expression_for_target(
                element,
                item,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx,
            )?;
        }
    }
    Ok(())
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
    for ((arg, expected_ty), borrow) in args
        .iter()
        .zip(sig.param_types.iter().cloned())
        .zip(sig.param_borrows.iter().copied())
    {
        match (borrow, arg) {
            (
                BorrowMode::Value,
                Expression::DirectBorrow(_) | Expression::ViewBorrow(_) | Expression::Move(_),
            ) => {
                return Err(sem_err(
                    SEM_ARG_TYPE,
                    format!(
                        "value parameter of '{}' must not use 'view', 'direct', or 'move' at call site.",
                        name
                    ),
                ));
            }
            (BorrowMode::DirectMutable, Expression::DirectBorrow(_)) => {}
            (BorrowMode::View, Expression::ViewBorrow(_)) => {}
            (BorrowMode::Move, Expression::Move(_)) => {}
            (BorrowMode::DirectMutable, _) => {
                return Err(sem_err(
                    SEM_ARG_TYPE,
                    format!(
                        "mutable borrowed parameter of '{}' requires explicit 'direct <identifier>' argument.",
                        name
                    ),
                ));
            }
            (BorrowMode::View, _) => {
                return Err(sem_err(
                    SEM_ARG_TYPE,
                    format!(
                        "read-only borrowed parameter of '{}' requires explicit 'view <identifier>' argument.",
                        name
                    ),
                ));
            }
            (BorrowMode::Move, _) => {
                return Err(sem_err(
                    SEM_ARG_TYPE,
                    format!(
                        "owning parameter of '{}' requires explicit 'move <identifier>' argument.",
                        name
                    ),
                ));
            }
            _ => {}
        }
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

fn resolve_function_name<'a>(
    raw_name: &'a str,
    scope: &HashMap<String, ValueType>,
    functions: &HashMap<String, FunctionSig>,
) -> Option<&'a str> {
    if functions.contains_key(raw_name) {
        return Some(raw_name);
    }
    if let Some((base, short)) = raw_name.split_once('.') {
        if base == "my" || scope.contains_key(base) {
            return None;
        }
        if functions.contains_key(short) {
            return Some(short);
        }
    }
    None
}

fn parse_declared_type_name(
    name: &str,
    structs: &HashMap<String, StructInfo>,
    nominal_types: &HashMap<String, ValueType>,
) -> ValueType {
    let resolve_struct_name = |raw: &str| -> Option<String> {
        if structs.contains_key(raw) {
            return Some(raw.to_string());
        }
        if let Some((_, short)) = raw.split_once('.')
            && structs.contains_key(short)
        {
            return Some(short.to_string());
        }
        None
    };

    if let Some(inner) = name.strip_prefix("Task(").and_then(|s| s.strip_suffix(')')) {
        return ValueType::Task(Some(Box::new(parse_declared_type_name(
            inner.trim(),
            structs,
            nominal_types,
        ))));
    }
    if name == "Task" {
        return ValueType::Task(None);
    }
    if let Some(inner) = name
        .strip_prefix("Channel(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return ValueType::Channel(Box::new(parse_declared_type_name(
            inner.trim(),
            structs,
            nominal_types,
        )));
    }
    if let Some(elem) = name.strip_suffix(" List") {
        let elem = elem.trim();
        let parsed_elem = parse_type_name(elem);
        if parsed_elem == ValueType::Unknown
            && let Some(resolved_struct) = resolve_struct_name(elem)
        {
            return ValueType::List(Box::new(ValueType::Struct(resolved_struct)));
        }
        return ValueType::List(Box::new(parsed_elem));
    }
    let parsed = parse_type_name(name);
    if parsed == ValueType::Unknown {
        let short = name.rsplit('.').next().unwrap_or(name);
        if let Some(nominal) = nominal_types.get(short) {
            return nominal.clone();
        }
    }
    if parsed == ValueType::Unknown
        && let Some(resolved_struct) = resolve_struct_name(name)
    {
        ValueType::Struct(resolved_struct)
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
        ValueType::List(_) | ValueType::Canvas | ValueType::Window | ValueType::Interrupt => false,
        _ => true,
    }
}

fn is_task_safe_boundary_type(
    ty: &ValueType,
    structs: &HashMap<String, StructInfo>,
    allow_channel: bool,
) -> bool {
    match ty {
        ValueType::Int
        | ValueType::Float
        | ValueType::Bool
        | ValueType::Char
        | ValueType::Text
        | ValueType::Time
        | ValueType::Duration
        | ValueType::ByteSize
        | ValueType::Angle
        | ValueType::Vec2
        | ValueType::Vec3
        | ValueType::Vec4
        | ValueType::Color
        | ValueType::Rect
        | ValueType::Label(_)
        | ValueType::Tag(_) => true,
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
        ValueType::List(_)
        | ValueType::Memory
        | ValueType::Interrupt
        | ValueType::Canvas
        | ValueType::Window
        | ValueType::Task(_)
        | ValueType::Unknown => false,
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
            size,
            kind,
            allow_grow,
            allow_drop: _,
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
            let size_ty = infer_expression_type(
                size,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            if size_ty != ValueType::ByteSize {
                return Err(err_at_code(
                    stmt,
                    SEM_MEMORY_RULE,
                    format!(
                        "memory(size) expects ByteSize, got {:?}. use a literal like '4kb' or a ByteSize value.",
                        size_ty
                    ),
                ));
            }
            match kind {
                crate::ast_nodes::MemoryKind::Child => {
                    if memory_state.active_memory.is_none() {
                        return Err(err_at_code(
                            stmt,
                            SEM_MEMORY_RULE,
                            "memory.child(size) is allowed only inside 'place in <parent> { ... }'."
                                .to_string(),
                        ));
                    }
                    if *allow_grow {
                        return Err(err_at_code(
                            stmt,
                            SEM_MEMORY_RULE,
                            "child Memory has fixed capacity borrowed from its parent and cannot use 'allow grow'."
                                .to_string(),
                        ));
                    }
                }
                crate::ast_nodes::MemoryKind::Static => {
                    if fn_ctx.is_some() {
                        return Err(err_at_code(
                            stmt,
                            SEM_MEMORY_RULE,
                            "memory.static(size) is allowed only at program root so its buffer cannot be aliased by recursive or concurrent function calls."
                                .to_string(),
                        ));
                    }
                    if !matches!(size.as_ref(), Expression::LiteralByteSize { bytes, .. } if *bytes > 0)
                    {
                        return Err(err_at_code(
                            stmt,
                            SEM_MEMORY_RULE,
                            "memory.static(size) requires a positive compile-time ByteSize literal."
                                .to_string(),
                        ));
                    }
                    if *allow_grow {
                        return Err(err_at_code(
                            stmt,
                            SEM_MEMORY_RULE,
                            "static Memory has fixed capacity and cannot use 'allow grow'."
                                .to_string(),
                        ));
                    }
                }
                crate::ast_nodes::MemoryKind::Dynamic => {}
            }
            scope.insert(name.clone(), ValueType::Memory);
            memory_state.memories.insert(
                name.clone(),
                MemoryBinding {
                    is_external: false,
                    is_cleared: false,
                    parent: (*kind == crate::ast_nodes::MemoryKind::Child)
                        .then(|| memory_state.active_memory.clone())
                        .flatten(),
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
            is_constant,
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
                let declared = if memory_state.labels.contains_key(tn) {
                    ValueType::Label(tn.clone())
                } else if memory_state.tags.contains_key(tn) {
                    ValueType::Tag(tn.clone())
                } else {
                    parse_declared_type_name(tn, structs, &HashMap::new())
                };
                validate_expression_for_target(
                    &declared,
                    value,
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx.as_ref(),
                )?;
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
            if declared_type.is_none()
                && matches!(
                    final_ty,
                    ValueType::List(_)
                        | ValueType::Struct(_)
                        | ValueType::Vec2
                        | ValueType::Vec3
                        | ValueType::Vec4
                        | ValueType::Color
                        | ValueType::Rect
                        | ValueType::Canvas
                        | ValueType::Window
                        | ValueType::Memory
                        | ValueType::Task(_)
                        | ValueType::Channel(_)
                        | ValueType::Unknown
                )
            {
                return Err(err_at_code(
                    stmt,
                    SEM_TYPE_MISMATCH,
                    format!(
                        "explicit type required for composite declaration '{}': inferred {:?}. use 'new <Type> {} = ...'.",
                        name, final_ty, name
                    ),
                ));
            }
            if is_movable_resource(&final_ty)
                && matches!(
                    value.as_ref(),
                    Expression::VariableReference(_)
                        | Expression::DirectBorrow(_)
                        | Expression::ViewBorrow(_)
                )
            {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!(
                        "resource declaration '{}' would copy ownership; use 'move <identifier>' or create a new resource.",
                        name
                    ),
                ));
            }
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
            if *is_constant
                && matches!(
                    final_ty,
                    ValueType::List(_)
                        | ValueType::Memory
                        | ValueType::Interrupt
                        | ValueType::Canvas
                        | ValueType::Window
                        | ValueType::Task(_)
                        | ValueType::Channel(_)
                )
            {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!(
                        "constant binding '{}' requires a value type; mutable capability {:?} is not allowed.",
                        name, final_ty
                    ),
                ));
            }
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
            if *is_constant {
                memory_state.constants.insert(name.clone());
            }
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
                memory_state.channel_owners.insert(name.clone());
            }
            if is_movable_resource(&final_ty) {
                memory_state.ownership.insert(
                    name.clone(),
                    OwnershipBinding {
                        state: OwnershipState::Owned,
                        moved_to: None,
                    },
                );
            }
            if matches!(final_ty, ValueType::Window | ValueType::Channel(_)) {
                let lifecycle = match value.as_ref() {
                    Expression::Move(source) => memory_state
                        .resource_lifecycles
                        .get(source)
                        .copied()
                        .unwrap_or(ResourceLifecycle::Open),
                    _ => ResourceLifecycle::Open,
                };
                memory_state
                    .resource_lifecycles
                    .insert(name.clone(), lifecycle);
            }
            if final_ty == ValueType::Interrupt {
                memory_state.interrupt_owners.insert(name.clone());
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
            if let Some(kind) = read_only_binding_kind(memory_state, target) {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!("{} '{}' cannot be reassigned.", kind, target),
                ));
            }
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
            if matches!(
                target_ty,
                ValueType::Canvas | ValueType::Window | ValueType::Interrupt
            ) {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!(
                        "resource capability '{}' cannot be reassigned or copied; pass it with 'direct' instead.",
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
            validate_expression_for_target(
                &target_ty,
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
        Statement::IncDec { target, .. } => {
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
            if let Some(kind) = read_only_binding_kind(memory_state, target) {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!("{} '{}' cannot be modified.", kind, target),
                ));
            }
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
            if object != "my"
                && let Some(kind) = read_only_binding_kind(memory_state, object)
            {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!("{} '{}' cannot be modified.", kind, object),
                ));
            }
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
            if vector_dimension(&owner_ty).is_some() {
                let valid_field = match &owner_ty {
                    ValueType::Vec2 => matches!(field.as_str(), "x" | "y"),
                    ValueType::Vec3 => matches!(field.as_str(), "x" | "y" | "z"),
                    ValueType::Vec4 => matches!(field.as_str(), "x" | "y" | "z" | "w"),
                    _ => false,
                };
                if !valid_field {
                    return Err(err_at_code(
                        stmt,
                        SEM_TYPE_MISMATCH,
                        format!("unknown vector field '{:?}.{}'.", owner_ty, field),
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
                if !is_numeric_type(&value_ty) {
                    return Err(err_at_code(
                        stmt,
                        SEM_TYPE_MISMATCH,
                        format!(
                            "vector field assignment '{}.{}' expects Int or Float, got {:?}.",
                            object, field, value_ty
                        ),
                    ));
                }
                record_task_effects_in_expr(stmt, value, scope, memory_state)?;
                return Ok(());
            }
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
            if info.hidden_fields.contains(field) && object != "my" {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    format!(
                        "field '{}.{}' is hidden; access it via methods of '{}'.",
                        owner, field, owner
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
            let Some(sig) = functions.get(name) else {
                return Err(sem_err(
                    SEM_INTERNAL,
                    format!("internal error: missing function signature for '{}'.", name),
                ));
            };
            let mut fn_scope = scope.clone();
            let mut fn_memory_state = MemoryState {
                labels: memory_state.labels.clone(),
                tags: memory_state.tags.clone(),
                ..MemoryState::default()
            };
            for (index, p) in params.iter().enumerate() {
                let pty = sig
                    .param_types
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| param_type_or_default(p));
                fn_scope.insert(p.name.clone(), pty.clone());
                if p.borrow == BorrowMode::View {
                    fn_memory_state.views.insert(p.name.clone());
                }
                if p.borrow == BorrowMode::Move && is_movable_resource(&pty) {
                    fn_memory_state.ownership.insert(
                        p.name.clone(),
                        OwnershipBinding {
                            state: OwnershipState::Owned,
                            moved_to: None,
                        },
                    );
                    if matches!(pty, ValueType::Window | ValueType::Channel(_)) {
                        fn_memory_state
                            .resource_lifecycles
                            .insert(p.name.clone(), ResourceLifecycle::Open);
                    }
                    if matches!(pty, ValueType::Channel(_)) {
                        fn_memory_state.channel_owners.insert(p.name.clone());
                    }
                    if pty == ValueType::Interrupt {
                        fn_memory_state.interrupt_owners.insert(p.name.clone());
                    }
                }
                if pty == ValueType::Memory {
                    fn_memory_state.memories.insert(
                        p.name.clone(),
                        MemoryBinding {
                            is_external: true,
                            is_cleared: false,
                            parent: None,
                        },
                    );
                }
                if let ValueType::Channel(elem_ty) = &pty {
                    fn_memory_state
                        .channels
                        .insert(p.name.clone(), (**elem_ty).clone());
                }
            }
            let local_ctx = FnContext {
                is_danger: sig.is_danger,
                return_type: sig.return_type.clone(),
                self_struct: None,
                is_task_context: task_context_functions.contains(name),
                is_timed_error_context: false,
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
            let original_memory_state = memory_state.clone();
            let mut then_scope = scope.clone();
            let mut then_memory_state = original_memory_state.clone();
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
            let then_terminates = block_guarantees_termination(then_block);
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
                let else_terminates = block_guarantees_termination(else_block);
                match (then_terminates, else_terminates) {
                    (false, false) => merge_resource_lifecycles(
                        memory_state,
                        &[&then_memory_state, &else_memory_state],
                        &original_memory_state,
                    ),
                    (false, true) => merge_resource_lifecycles(
                        memory_state,
                        &[&then_memory_state],
                        &original_memory_state,
                    ),
                    (true, false) => merge_resource_lifecycles(
                        memory_state,
                        &[&else_memory_state],
                        &original_memory_state,
                    ),
                    (true, true) => {}
                }
            } else if !then_terminates {
                merge_resource_lifecycles(
                    memory_state,
                    &[&then_memory_state, &original_memory_state],
                    &original_memory_state,
                );
            }
            Ok(())
        }
        Statement::ForLoop {
            initialization,
            condition,
            update,
            style,
            body,
            ..
        } => {
            if *style == ForLoopStyle::LegacyCStyle {
                return Err(err_at_code(
                    stmt,
                    SEM_INVALID_CONTEXT,
                    "unsupported context: legacy 'for (init; condition; update)' is parse/format compatibility syntax only in v1.2; use 'iterate collection as item' or 'for item in collection'."
                        .to_string(),
                ));
            }
            let original_memory_state = memory_state.clone();
            let mut loop_scope = scope.clone();
            let mut loop_memory_state = original_memory_state.clone();
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
            )?;
            if !block_guarantees_termination(body) {
                ensure_loop_preserves_ownership(stmt, &original_memory_state, &loop_memory_state)?;
            }
            merge_resource_lifecycles(
                memory_state,
                &[&original_memory_state, &loop_memory_state],
                &original_memory_state,
            );
            Ok(())
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            let original_memory_state = memory_state.clone();
            let when_ty = infer_expression_type(
                when_expression,
                scope,
                memory_state,
                functions,
                structs,
                fn_ctx.as_ref(),
            )?;
            let mut branch_states = Vec::new();
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
                let mut case_memory_state = original_memory_state.clone();
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
                if !block_guarantees_termination(block) {
                    branch_states.push(case_memory_state);
                }
            }
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                let mut else_memory_state = original_memory_state.clone();
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
                if !block_guarantees_termination(else_block) {
                    branch_states.push(else_memory_state);
                }
            } else {
                branch_states.push(original_memory_state.clone());
            }
            if !branch_states.is_empty() {
                let refs = branch_states.iter().collect::<Vec<_>>();
                merge_resource_lifecycles(memory_state, &refs, &original_memory_state);
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
            let original_memory_state = memory_state.clone();
            let mut while_scope = scope.clone();
            let mut while_memory_state = original_memory_state.clone();
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
            )?;
            if !block_guarantees_termination(body) {
                ensure_loop_preserves_ownership(stmt, &original_memory_state, &while_memory_state)?;
            }
            merge_resource_lifecycles(
                memory_state,
                &[&original_memory_state, &while_memory_state],
                &original_memory_state,
            );
            Ok(())
        }
        Statement::LoopStatement { body, .. } => {
            let original_memory_state = memory_state.clone();
            let mut local_scope = scope.clone();
            let mut local_memory_state = original_memory_state.clone();
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
            )?;
            if !block_guarantees_termination(body) {
                ensure_loop_preserves_ownership(stmt, &original_memory_state, &local_memory_state)?;
            }
            merge_resource_lifecycles(
                memory_state,
                &[&original_memory_state, &local_memory_state],
                &original_memory_state,
            );
            Ok(())
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
            let mut cleared = HashSet::from([memory_name.clone()]);
            loop {
                let before = cleared.len();
                for (name, binding) in &memory_state.memories {
                    if binding
                        .parent
                        .as_ref()
                        .map(|parent| cleared.contains(parent))
                        .unwrap_or(false)
                    {
                        cleared.insert(name.clone());
                    }
                }
                if cleared.len() == before {
                    break;
                }
            }
            for name in cleared {
                if let Some(binding) = memory_state.memories.get_mut(&name) {
                    binding.is_cleared = true;
                }
            }
            Ok(())
        }
        Statement::OnBlock {
            trigger,
            target,
            body,
            ..
        } => {
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
            if trigger == "interrupt" {
                if fn_ctx.is_some() || in_loop || memory_state.active_memory.is_some() {
                    return Err(err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
                        "on interrupt registration is allowed only at project top level in the host MVP."
                            .to_string(),
                    ));
                }
                let Some(target) = target.as_deref() else {
                    return Err(err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
                        "on interrupt requires an Interrupt capability identifier.".to_string(),
                    ));
                };
                if scope.get(target) != Some(&ValueType::Interrupt)
                    || !memory_state.interrupt_owners.contains(target)
                {
                    return Err(err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
                        format!(
                            "on interrupt target '{}' must be an owning Interrupt capability.",
                            target
                        ),
                    ));
                }
                validate_interrupt_block(body, memory_state)?;
                let mut handler_scope = scope.clone();
                let mut handler_state = memory_state.clone();
                analyze_block(
                    body,
                    &mut handler_scope,
                    &mut handler_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    fn_ctx,
                    false,
                )?;
                return Ok(());
            }
            Err(err_at_code(
                stmt,
                SEM_INVALID_CONTEXT,
                format!("unsupported on-block trigger '{}'.", trigger),
            ))
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
            if call_name == "__task_wait_for" {
                let (task_name, result_type) = validate_timed_task_wait(
                    stmt,
                    args,
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx.as_ref(),
                )?;
                let Some(result_type) = result_type else {
                    return Err(err_at_code(
                        stmt,
                        SEM_TASK_RULE,
                        format!(
                            "timed wait for void task '{}' cannot assign a result.",
                            task_name
                        ),
                    ));
                };
                let Some(target_type) = scope.get(target) else {
                    return Err(err_at_code(
                        stmt,
                        SEM_USE_BEFORE_DEF,
                        format!("use-before-definition: '{}' is not defined.", target),
                    ));
                };
                if let Some(kind) = read_only_binding_kind(memory_state, target) {
                    return Err(err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
                        format!("{} '{}' cannot be reassigned.", kind, target),
                    ));
                }
                if !can_assign(target_type, &result_type) {
                    return Err(err_at_code(
                        stmt,
                        SEM_TYPE_MISMATCH,
                        format!(
                            "timed wait target mismatch: '{}' expects {:?}, task returns {:?}.",
                            target, target_type, result_type
                        ),
                    ));
                }
                let mut handler_state = memory_state.clone();
                memory_state
                    .tasks
                    .get_mut(&task_name)
                    .expect("validated timed task must exist")
                    .waited = true;
                let mut handler_scope = scope.clone();
                return analyze_block(
                    on_error,
                    &mut handler_scope,
                    &mut handler_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    timed_error_context(fn_ctx, true),
                    in_loop,
                );
            }
            if let Some((channel, method @ ("receive" | "receive_for"))) = call_name.split_once('.')
                && let Some(element_ty) = memory_state.channels.get(channel).cloned()
            {
                require_owned_resource(memory_state, channel)?;
                let expected_args = usize::from(method == "receive_for");
                if args.len() != expected_args {
                    return Err(err_at_code(
                        stmt,
                        SEM_CHANNEL_RULE,
                        format!(
                            "Channel.{method} expects {expected_args} argument(s), got {}.",
                            args.len()
                        ),
                    ));
                }
                if method == "receive_for" {
                    let timeout_ty = infer_expression_type(
                        &args[0],
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx.as_ref(),
                    )?;
                    if timeout_ty != ValueType::Duration {
                        return Err(err_at_code(
                            stmt,
                            SEM_CHANNEL_RULE,
                            format!(
                                "Channel.receive_for expects Duration, got {:?}.",
                                timeout_ty
                            ),
                        ));
                    }
                }
                let Some(target_ty) = scope.get(target) else {
                    return Err(err_at_code(
                        stmt,
                        SEM_USE_BEFORE_DEF,
                        format!("use-before-definition: '{}' is not defined.", target),
                    ));
                };
                if !can_assign(target_ty, &element_ty) {
                    return Err(err_at_code(
                        stmt,
                        SEM_CHANNEL_RULE,
                        format!(
                            "channel receive target mismatch: '{}' expects {:?}, channel carries {:?}.",
                            target, target_ty, element_ty
                        ),
                    ));
                }
                let mut on_error_scope = scope.clone();
                let mut on_error_memory_state = memory_state.clone();
                return analyze_block(
                    on_error,
                    &mut on_error_scope,
                    &mut on_error_memory_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    timed_error_context(fn_ctx, method == "receive_for"),
                    in_loop,
                );
            }
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
            let Some(resolved_name) = resolve_function_name(call_name, scope, functions) else {
                return Err(err_at_code(
                    stmt,
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in on error call.", call_name),
                ));
            };
            let sig = functions
                .get(resolved_name)
                .expect("resolved function must exist");
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
                resolved_name,
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
            if call_name == "__task_wait_for" {
                let (task_name, _) = validate_timed_task_wait(
                    stmt,
                    args,
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx.as_ref(),
                )?;
                let mut handler_state = memory_state.clone();
                memory_state
                    .tasks
                    .get_mut(&task_name)
                    .expect("validated timed task must exist")
                    .waited = true;
                let mut handler_scope = scope.clone();
                return analyze_block(
                    on_error,
                    &mut handler_scope,
                    &mut handler_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    timed_error_context(fn_ctx, true),
                    in_loop,
                );
            }
            if let Some((window, method @ ("present" | "close"))) = call_name.split_once('.')
                && matches!(scope.get(window), Some(ValueType::Window))
            {
                require_owned_resource(memory_state, window)?;
                if memory_state.views.contains(window) {
                    return Err(err_at_code(
                        stmt,
                        SEM_INVALID_CONTEXT,
                        format!(
                            "view parameter '{}' cannot call mutating method '{}'.",
                            window, method
                        ),
                    ));
                }
                if method == "close" {
                    if !args.is_empty() {
                        return Err(err_at_code(
                            stmt,
                            SEM_INVALID_CONTEXT,
                            "Window.close() does not accept arguments.".to_string(),
                        ));
                    }
                    if !memory_state.resource_lifecycles.contains_key(window) {
                        return Err(err_at_code(
                            stmt,
                            SEM_INVALID_CONTEXT,
                            format!(
                                "only owning Window binding may close '{}'; borrowed window parameters cannot close their owner.",
                                window
                            ),
                        ));
                    }
                    memory_state
                        .resource_lifecycles
                        .insert(window.to_string(), ResourceLifecycle::Closed);
                } else {
                    if !matches!(args.as_slice(), [Expression::DirectBorrow(_)]) {
                        return Err(err_at_code(
                            stmt,
                            SEM_ARG_TYPE,
                            "Window.present requires explicit 'direct <Canvas>' borrow."
                                .to_string(),
                        ));
                    }
                    let actual_ty = infer_expression_type(
                        &args[0],
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx.as_ref(),
                    )?;
                    if actual_ty != ValueType::Canvas {
                        return Err(err_at_code(
                            stmt,
                            SEM_ARG_TYPE,
                            format!("Window.present expects Canvas, got {:?}.", actual_ty),
                        ));
                    }
                }
                let mut on_error_scope = scope.clone();
                let mut on_error_memory_state = memory_state.clone();
                return analyze_block(
                    on_error,
                    &mut on_error_scope,
                    &mut on_error_memory_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    fn_ctx,
                    in_loop,
                );
            }
            if let Some((channel, "close")) = call_name.split_once('.')
                && memory_state.channels.contains_key(channel)
            {
                require_owned_resource(memory_state, channel)?;
                if !args.is_empty() {
                    return Err(err_at_code(
                        stmt,
                        SEM_CHANNEL_RULE,
                        "Channel.close() does not accept arguments.".to_string(),
                    ));
                }
                if !memory_state.channel_owners.contains(channel) {
                    return Err(err_at_code(
                        stmt,
                        SEM_CHANNEL_RULE,
                        format!(
                            "only owning Channel binding may close '{}'; borrowed channel parameters cannot close their owner.",
                            channel
                        ),
                    ));
                }
                memory_state
                    .resource_lifecycles
                    .insert(channel.to_string(), ResourceLifecycle::Closed);
                let mut on_error_scope = scope.clone();
                let mut on_error_memory_state = memory_state.clone();
                return analyze_block(
                    on_error,
                    &mut on_error_scope,
                    &mut on_error_memory_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    fn_ctx,
                    in_loop,
                );
            }
            if let Some((channel, method @ ("send" | "send_for"))) = call_name.split_once('.')
                && let Some(element_ty) = memory_state.channels.get(channel).cloned()
            {
                require_owned_resource(memory_state, channel)?;
                let expected_args = if method == "send_for" { 2 } else { 1 };
                if args.len() != expected_args {
                    return Err(err_at_code(
                        stmt,
                        SEM_CHANNEL_RULE,
                        format!(
                            "Channel.{method} expects {expected_args} argument(s), got {}.",
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
                    fn_ctx.as_ref(),
                )?;
                if !can_assign(&element_ty, &actual_ty) {
                    return Err(err_at_code(
                        stmt,
                        SEM_CHANNEL_RULE,
                        format!(
                            "channel send type mismatch: expected {:?}, got {:?}.",
                            element_ty, actual_ty
                        ),
                    ));
                }
                if method == "send_for" {
                    let timeout_ty = infer_expression_type(
                        &args[1],
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx.as_ref(),
                    )?;
                    if timeout_ty != ValueType::Duration {
                        return Err(err_at_code(
                            stmt,
                            SEM_CHANNEL_RULE,
                            format!("Channel.send_for expects Duration, got {:?}.", timeout_ty),
                        ));
                    }
                }
                let mut on_error_scope = scope.clone();
                let mut on_error_memory_state = memory_state.clone();
                return analyze_block(
                    on_error,
                    &mut on_error_scope,
                    &mut on_error_memory_state,
                    functions,
                    labels,
                    structs,
                    task_context_functions,
                    timed_error_context(fn_ctx, method == "send_for"),
                    in_loop,
                );
            }
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
            let Some(resolved_name) = resolve_function_name(call_name, scope, functions) else {
                return Err(err_at_code(
                    stmt,
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in on error call.", call_name),
                ));
            };
            let sig = functions
                .get(resolved_name)
                .expect("resolved function must exist");
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
                resolved_name,
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
                    validate_expression_for_target(
                        &elem_ty,
                        value,
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx.as_ref(),
                    )?;
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
            let variant = code.split('.').next_back().unwrap_or(code.as_str());
            if !error_codes.iter().any(|v| v == variant) {
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
                    if is_movable_resource(&actual) && !matches!(expr.as_ref(), Expression::Move(_))
                    {
                        return Err(err_at_code(
                            stmt,
                            SEM_INVALID_CONTEXT,
                            "returning an owning resource requires explicit 'return move <identifier>'."
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
                    if let Some(expected) = ctx.return_type.as_ref() {
                        validate_expression_for_target(
                            expected,
                            expr,
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx.as_ref(),
                        )?;
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
            if let Expression::Call { name, .. } = expr.as_ref()
                && let Some((resource, "close")) = name.split_once('.')
                && memory_state.resource_lifecycles.contains_key(resource)
            {
                memory_state
                    .resource_lifecycles
                    .insert(resource.to_string(), ResourceLifecycle::Closed);
            }
            record_task_effects_in_expr(stmt, expr, scope, memory_state)?;
            Ok(())
        }
        Statement::StructDecl { name, methods, .. } => {
            for m in methods {
                let mut method_scope = scope.clone();
                let mut method_memory_state = MemoryState {
                    labels: memory_state.labels.clone(),
                    tags: memory_state.tags.clone(),
                    ..MemoryState::default()
                };
                method_scope.insert("my".to_string(), ValueType::Struct(name.clone()));
                for p in &m.params {
                    let pty = param_type_or_default(p);
                    method_scope.insert(p.name.clone(), pty.clone());
                    if p.borrow == BorrowMode::View {
                        method_memory_state.views.insert(p.name.clone());
                    }
                    if p.borrow == BorrowMode::Move && is_movable_resource(&pty) {
                        method_memory_state.ownership.insert(
                            p.name.clone(),
                            OwnershipBinding {
                                state: OwnershipState::Owned,
                                moved_to: None,
                            },
                        );
                        if matches!(pty, ValueType::Window | ValueType::Channel(_)) {
                            method_memory_state
                                .resource_lifecycles
                                .insert(p.name.clone(), ResourceLifecycle::Open);
                        }
                    }
                    if pty == ValueType::Memory {
                        method_memory_state.memories.insert(
                            p.name.clone(),
                            MemoryBinding {
                                is_external: true,
                                is_cleared: false,
                                parent: None,
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
                    is_timed_error_context: false,
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
        Statement::LabelDecl { .. } | Statement::TagDecl { .. } => Ok(()),
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

fn timed_error_context(fn_ctx: Option<FnContext>, enabled: bool) -> Option<FnContext> {
    if !enabled {
        return fn_ctx;
    }
    let mut context = fn_ctx.unwrap_or(FnContext {
        is_danger: false,
        return_type: None,
        self_struct: None,
        is_task_context: false,
        is_timed_error_context: false,
    });
    context.is_timed_error_context = true;
    Some(context)
}

#[allow(clippy::too_many_arguments)]
fn validate_timed_task_wait(
    stmt: &Statement,
    args: &[Expression],
    scope: &HashMap<String, ValueType>,
    memory_state: &MemoryState,
    functions: &HashMap<String, FunctionSig>,
    structs: &HashMap<String, StructInfo>,
    fn_ctx: Option<&FnContext>,
) -> Result<(String, Option<ValueType>), String> {
    let [Expression::VariableReference(task_name), timeout] = args else {
        return Err(err_at_code(
            stmt,
            SEM_TASK_RULE,
            "internal timed wait shape is invalid.".to_string(),
        ));
    };
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
            format!("timed wait expects Task handle, got {:?}.", task_ty),
        ));
    }
    let Some(binding) = memory_state.tasks.get(task_name) else {
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
    let timeout_ty =
        infer_expression_type(timeout, scope, memory_state, functions, structs, fn_ctx)?;
    if timeout_ty != ValueType::Duration {
        return Err(err_at_code(
            stmt,
            SEM_TASK_RULE,
            format!("timed wait expects Duration, got {:?}.", timeout_ty),
        ));
    }
    Ok((task_name.clone(), binding.result_type.clone()))
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
        Expression::WaitTask { task_name, .. } => {
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
        Expression::Call { name, args } => {
            for arg in args {
                if let Expression::Move(resource) = arg {
                    mark_resource_moved(memory_state, resource, name.clone())?;
                } else {
                    record_task_effects_in_expr(stmt, arg, scope, memory_state)?;
                }
            }
            Ok(())
        }
        Expression::RunTask { call_name, args } => {
            for arg in args {
                if let Expression::Move(resource) = arg {
                    mark_resource_moved(memory_state, resource, format!("task '{}'", call_name))?;
                } else {
                    record_task_effects_in_expr(stmt, arg, scope, memory_state)?;
                }
            }
            Ok(())
        }
        Expression::Move(name) => mark_resource_moved(memory_state, name, "new owner"),
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
        | Expression::DirectBorrow(_)
        | Expression::ViewBorrow(_)
        | Expression::MemberAccess { .. }
        | Expression::Stopping
        | Expression::TimedOut
        | Expression::LiteralInt(_)
        | Expression::LiteralFloat(_)
        | Expression::LiteralBool(_)
        | Expression::LiteralChar(_)
        | Expression::LiteralString(_)
        | Expression::LiteralDuration { .. }
        | Expression::LiteralByteSize { .. }
        | Expression::LiteralAngle { .. } => Ok(()),
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
        Expression::WaitTask { task_name, .. } => {
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
        | Expression::LiteralChar(_)
        | Expression::LiteralString(_)
        | Expression::LiteralDuration { .. }
        | Expression::LiteralByteSize { .. }
        | Expression::LiteralAngle { .. }
        | Expression::VariableReference(_)
        | Expression::DirectBorrow(_)
        | Expression::ViewBorrow(_)
        | Expression::Move(_)
        | Expression::MemberAccess { .. }
        | Expression::Stopping
        | Expression::TimedOut => Ok(()),
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
        Expression::WaitTask { task_name, timeout } => {
            names.contains(task_name)
                || timeout
                    .as_deref()
                    .map(|timeout| expression_uses_task_handle(timeout, names))
                    .unwrap_or(false)
        }
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
        Statement::DangerAssignOnError {
            call_name,
            args,
            on_error,
            ..
        }
        | Statement::DangerCallOnError {
            call_name,
            args,
            on_error,
            ..
        } => {
            let mut outputs = Vec::new();
            for mut state in states {
                if call_name == "__task_wait_for" {
                    let [Expression::VariableReference(task_name), timeout] = args.as_slice()
                    else {
                        return Err(err_at_code(
                            stmt,
                            SEM_TASK_RULE,
                            "internal timed wait shape is invalid.".to_string(),
                        ));
                    };
                    apply_task_flow_expression(stmt, timeout, &mut state)?;
                    if !state.contains_key(task_name) {
                        return Err(err_at_code(
                            stmt,
                            SEM_TASK_RULE,
                            format!(
                                "task handle '{}' is not live on every path at this timed wait.",
                                task_name
                            ),
                        ));
                    }

                    let mut success_state = state.clone();
                    success_state.remove(task_name);
                    outputs.push(success_state);

                    let error_outputs =
                        process_task_flow_statements(&on_error.statements, vec![state.clone()])?;
                    ensure_no_new_task_bindings(&state, &error_outputs)?;
                    outputs.extend(error_outputs);
                    continue;
                }
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
        | Statement::LabelDecl { .. }
        | Statement::TagDecl { .. } => Ok(states),
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
        Expression::LiteralChar(_) => Ok(ValueType::Char),
        Expression::LiteralString(_) => Ok(ValueType::Text),
        Expression::LiteralDuration { .. } => Ok(ValueType::Duration),
        Expression::LiteralByteSize { .. } => Ok(ValueType::ByteSize),
        Expression::LiteralAngle { .. } => Ok(ValueType::Angle),
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
                require_owned_resource(memory_state, name)?;
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
        Expression::DirectBorrow(name) => {
            require_owned_resource(memory_state, name)?;
            scope.get(name).cloned().ok_or_else(|| {
                sem_err(
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined for direct borrow.",
                        name
                    ),
                )
            })
        }
        Expression::ViewBorrow(name) => {
            require_owned_resource(memory_state, name)?;
            scope.get(name).cloned().ok_or_else(|| {
                sem_err(
                    SEM_USE_BEFORE_DEF,
                    format!(
                        "use-before-definition: '{}' is not defined for view borrow.",
                        name
                    ),
                )
            })
        }
        Expression::Move(name) => {
            require_owned_resource(memory_state, name)?;
            let ty = scope.get(name).cloned().ok_or_else(|| {
                sem_err(
                    SEM_USE_BEFORE_DEF,
                    format!("use-before-definition: '{}' is not defined for move.", name),
                )
            })?;
            if !is_movable_resource(&ty) {
                return Err(sem_err(
                    SEM_INVALID_CONTEXT,
                    format!(
                        "'move {}' requires an owning resource, got {:?}; ordinary values are copied.",
                        name, ty
                    ),
                ));
            }
            Ok(ty)
        }
        Expression::MemberAccess { base, .. } => {
            if let Expression::MemberAccess { field, .. } = expr {
                if base == "Color" && is_color_constant(field) {
                    return Ok(ValueType::Color);
                }
                if memory_state
                    .labels
                    .get(base)
                    .map(|variants| variants.contains(field))
                    .unwrap_or(false)
                {
                    return Ok(ValueType::Label(base.clone()));
                }
                if memory_state
                    .tags
                    .get(base)
                    .map(|variants| variants.contains(field))
                    .unwrap_or(false)
                {
                    return Ok(ValueType::Tag(base.clone()));
                }
            }
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
                require_owned_resource(memory_state, base)?;
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
                if vector_dimension(&owner_ty).is_some() {
                    let valid_field = match &owner_ty {
                        ValueType::Vec2 => matches!(field.as_str(), "x" | "y"),
                        ValueType::Vec3 => matches!(field.as_str(), "x" | "y" | "z"),
                        ValueType::Vec4 => matches!(field.as_str(), "x" | "y" | "z" | "w"),
                        _ => false,
                    };
                    if valid_field {
                        return Ok(ValueType::Float);
                    }
                    return Err(sem_err(
                        SEM_TYPE_MISMATCH,
                        format!("unknown vector field '{:?}.{}'.", owner_ty, field),
                    ));
                }
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
                if info.hidden_fields.contains(field) && base != "my" {
                    return Err(sem_err(
                        SEM_INVALID_CONTEXT,
                        format!(
                            "field '{}.{}' is hidden; access it via methods of '{}'.",
                            owner, field, owner
                        ),
                    ));
                }
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
            if let Some((receiver, _)) = name.split_once('.')
                && scope.contains_key(receiver)
            {
                require_owned_resource(memory_state, receiver)?;
            }
            if matches!(name.as_str(), "color" | "color_hex" | "rect" | "canvas")
                || name == "windows.open"
            {
                let (expected, result) = match name.as_str() {
                    "color" => (
                        vec![
                            ValueType::Int,
                            ValueType::Int,
                            ValueType::Int,
                            ValueType::Int,
                        ],
                        ValueType::Color,
                    ),
                    "color_hex" => (vec![ValueType::Text, ValueType::Int], ValueType::Color),
                    "rect" => (
                        vec![
                            ValueType::Float,
                            ValueType::Float,
                            ValueType::Float,
                            ValueType::Float,
                        ],
                        ValueType::Rect,
                    ),
                    "canvas" => (vec![ValueType::Int, ValueType::Int], ValueType::Canvas),
                    _ => (
                        vec![ValueType::Text, ValueType::Int, ValueType::Int],
                        ValueType::Window,
                    ),
                };
                if args.len() != expected.len() {
                    return Err(sem_err(
                        SEM_ARG_COUNT,
                        format!(
                            "visual builtin '{}' expects {} arguments, got {}.",
                            name,
                            expected.len(),
                            args.len()
                        ),
                    ));
                }
                for (index, (arg, expected_ty)) in args.iter().zip(expected).enumerate() {
                    let actual = infer_expression_type(
                        arg,
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx,
                    )?;
                    if !can_assign(&expected_ty, &actual) {
                        return Err(sem_err(
                            SEM_ARG_TYPE,
                            format!(
                                "visual builtin '{}' argument {} expects {:?}, got {:?}.",
                                name,
                                index + 1,
                                expected_ty,
                                actual
                            ),
                        ));
                    }
                }
                return Ok(result);
            }
            if name == "interrupts.periodic" {
                if args.len() != 1 {
                    return Err(sem_err(
                        SEM_ARG_COUNT,
                        format!(
                            "interrupts.periodic expects one Duration argument, got {}.",
                            args.len()
                        ),
                    ));
                }
                let duration_ty = infer_expression_type(
                    &args[0],
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx,
                )?;
                if duration_ty != ValueType::Duration {
                    return Err(sem_err(
                        SEM_ARG_TYPE,
                        format!(
                            "interrupts.periodic expects Duration, got {:?}.",
                            duration_ty
                        ),
                    ));
                }
                return Ok(ValueType::Interrupt);
            }
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
                if let Some(receiver_ty) = scope.get(base) {
                    if memory_state.views.contains(base)
                        && (matches!(receiver_ty, ValueType::Canvas) && method != "checksum"
                            || matches!(receiver_ty, ValueType::Window))
                    {
                        return Err(sem_err(
                            SEM_INVALID_CONTEXT,
                            format!(
                                "view parameter '{}' cannot call mutating method '{}'.",
                                base, method
                            ),
                        ));
                    }
                    if matches!(receiver_ty, ValueType::Window)
                        && matches!(method, "present" | "close")
                    {
                        if method == "close" && !memory_state.resource_lifecycles.contains_key(base)
                        {
                            return Err(sem_err(
                                SEM_INVALID_CONTEXT,
                                format!(
                                    "only owning Window binding may close '{}'; borrowed window parameters cannot close their owner.",
                                    base
                                ),
                            ));
                        }
                        require_open_resource(memory_state, base, method)?;
                    }
                    let expected = match (receiver_ty, method) {
                        (ValueType::Canvas, "clear") => {
                            Some((vec![ValueType::Color], ValueType::Int))
                        }
                        (ValueType::Canvas, "pixel") => {
                            Some((vec![ValueType::Vec2, ValueType::Color], ValueType::Int))
                        }
                        (ValueType::Canvas, "line") => Some((
                            vec![ValueType::Vec2, ValueType::Vec2, ValueType::Color],
                            ValueType::Int,
                        )),
                        (ValueType::Canvas, "rect" | "fill_rect") => {
                            Some((vec![ValueType::Rect, ValueType::Color], ValueType::Int))
                        }
                        (ValueType::Canvas, "circle" | "fill_circle") => Some((
                            vec![ValueType::Vec2, ValueType::Float, ValueType::Color],
                            ValueType::Int,
                        )),
                        (ValueType::Canvas, "checksum") => Some((vec![], ValueType::Int)),
                        (ValueType::Window, "present") => {
                            if !matches!(args.as_slice(), [Expression::DirectBorrow(_)]) {
                                return Err(sem_err(
                                    SEM_ARG_TYPE,
                                    "Window.present requires explicit 'direct <Canvas>' borrow."
                                        .to_string(),
                                ));
                            }
                            Some((vec![ValueType::Canvas], ValueType::Int))
                        }
                        (ValueType::Window, "is_open") => Some((vec![], ValueType::Bool)),
                        (ValueType::Window, "close") => Some((vec![], ValueType::Int)),
                        _ => None,
                    };
                    if let Some((expected, result)) = expected {
                        if args.len() != expected.len() {
                            return Err(sem_err(
                                SEM_ARG_COUNT,
                                format!(
                                    "visual method '{}.{}' expects {} arguments, got {}.",
                                    base,
                                    method,
                                    expected.len(),
                                    args.len()
                                ),
                            ));
                        }
                        for (index, (arg, expected_ty)) in args.iter().zip(expected).enumerate() {
                            let actual = infer_expression_type(
                                arg,
                                scope,
                                memory_state,
                                functions,
                                structs,
                                fn_ctx,
                            )?;
                            if !can_assign(&expected_ty, &actual) {
                                return Err(sem_err(
                                    SEM_ARG_TYPE,
                                    format!(
                                        "visual method '{}.{}' argument {} expects {:?}, got {:?}.",
                                        base,
                                        method,
                                        index + 1,
                                        expected_ty,
                                        actual
                                    ),
                                ));
                            }
                        }
                        return Ok(result);
                    }
                    if matches!(receiver_ty, ValueType::Canvas | ValueType::Window) {
                        return Err(sem_err(
                            SEM_UNKNOWN_FUNCTION,
                            format!("unknown visual method '{}.{}'.", base, method),
                        ));
                    }
                }
                if matches!(
                    method,
                    "send" | "receive" | "send_for" | "receive_for" | "try_send" | "close"
                ) && !memory_state.channels.contains_key(base)
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
                    if matches!(
                        method,
                        "send" | "receive" | "send_for" | "receive_for" | "close"
                    ) {
                        if method == "close" && !memory_state.channel_owners.contains(base) {
                            return Err(sem_err(
                                SEM_CHANNEL_RULE,
                                format!(
                                    "only owning Channel binding may close '{}'; borrowed channel parameters cannot close their owner.",
                                    base
                                ),
                            ));
                        }
                        require_open_resource(memory_state, base, method)?;
                    }
                    return match method {
                        "send" | "send_for" | "try_send" => {
                            let expected_args = if method == "send_for" { 2 } else { 1 };
                            if args.len() != expected_args {
                                return Err(sem_err(
                                    SEM_CHANNEL_RULE,
                                    format!(
                                        "{} expects {} argument(s), got {}.",
                                        method,
                                        expected_args,
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
                            if method == "send_for" {
                                let timeout_ty = infer_expression_type(
                                    &args[1],
                                    scope,
                                    memory_state,
                                    functions,
                                    structs,
                                    fn_ctx,
                                )?;
                                if timeout_ty != ValueType::Duration {
                                    return Err(sem_err(
                                        SEM_CHANNEL_RULE,
                                        format!(
                                            "Channel.send_for expects Duration, got {:?}.",
                                            timeout_ty
                                        ),
                                    ));
                                }
                                return Err(sem_err(
                                    SEM_INVALID_CONTEXT,
                                    "Channel.send_for requires an 'on error' handler.".to_string(),
                                ));
                            }
                            if method == "try_send" {
                                Ok(ValueType::Bool)
                            } else {
                                Ok(ValueType::Int)
                            }
                        }
                        "receive" | "receive_for" => {
                            let expected_args = usize::from(method == "receive_for");
                            if args.len() != expected_args {
                                return Err(sem_err(
                                    SEM_CHANNEL_RULE,
                                    format!(
                                        "{} expects {} argument(s), got {}.",
                                        method,
                                        expected_args,
                                        args.len()
                                    ),
                                ));
                            }
                            if method == "receive_for" {
                                let timeout_ty = infer_expression_type(
                                    &args[0],
                                    scope,
                                    memory_state,
                                    functions,
                                    structs,
                                    fn_ctx,
                                )?;
                                if timeout_ty != ValueType::Duration {
                                    return Err(sem_err(
                                        SEM_CHANNEL_RULE,
                                        format!(
                                            "Channel.receive_for expects Duration, got {:?}.",
                                            timeout_ty
                                        ),
                                    ));
                                }
                                return Err(sem_err(
                                    SEM_INVALID_CONTEXT,
                                    "Channel.receive_for requires an 'on error' handler."
                                        .to_string(),
                                ));
                            }
                            Ok(channel_elem_ty)
                        }
                        "close" => {
                            if !args.is_empty() {
                                return Err(sem_err(
                                    SEM_CHANNEL_RULE,
                                    format!("close expects no arguments, got {}.", args.len()),
                                ));
                            }
                            Ok(ValueType::Int)
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
                    Builtin::Now => Ok(ValueType::Time),
                    Builtin::Elapsed => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if ty != ValueType::Time {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin 'elapsed' expects (Time), got ({:?}).", ty),
                            ));
                        }
                        Ok(ValueType::Duration)
                    }
                    Builtin::Sleep | Builtin::Delay => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if ty != ValueType::Duration {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin '{}' expects (Duration), got ({:?}).", name, ty),
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
                    Builtin::Floor | Builtin::Ceil | Builtin::Round | Builtin::Sqrt => {
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
                    Builtin::Sin | Builtin::Cos => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if ty != ValueType::Angle && !is_numeric_type(&ty) {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin '{}' expects Angle or legacy numeric radians, got {:?}.",
                                    name, ty
                                ),
                            ));
                        }
                        Ok(ValueType::Float)
                    }
                    Builtin::DegToRad => {
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
                                    "builtin '{}' expects numeric degrees, got {:?}.",
                                    name, ty
                                ),
                            ));
                        }
                        Ok(ValueType::Angle)
                    }
                    Builtin::RadToDeg => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if ty != ValueType::Angle {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!("builtin '{}' expects Angle, got {:?}.", name, ty),
                            ));
                        }
                        Ok(ValueType::Float)
                    }
                    Builtin::Atan2 => {
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
                        Ok(ValueType::Angle)
                    }
                    Builtin::Root => {
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
                    Builtin::Dot | Builtin::Distance | Builtin::DistanceSq => {
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
                        if vector_dimension(&a).is_none() || a != b {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin '{}' expects two vectors of the same dimension, got ({:?}, {:?}).",
                                    name, a, b
                                ),
                            ));
                        }
                        Ok(ValueType::Float)
                    }
                    Builtin::Length | Builtin::LengthSq => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if vector_dimension(&ty).is_none() {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin '{}' expects Vec2, Vec3, or Vec4, got {:?}.",
                                    name, ty
                                ),
                            ));
                        }
                        Ok(ValueType::Float)
                    }
                    Builtin::Normalize => {
                        let ty = infer_expression_type(
                            &args[0],
                            scope,
                            memory_state,
                            functions,
                            structs,
                            fn_ctx,
                        )?;
                        if vector_dimension(&ty).is_none() {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin 'normalize' expects Vec2, Vec3, or Vec4, got {:?}.",
                                    ty
                                ),
                            ));
                        }
                        Ok(ty)
                    }
                    Builtin::Cross => {
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
                        if a != ValueType::Vec3 || b != ValueType::Vec3 {
                            return Err(sem_err(
                                SEM_TYPE_MISMATCH,
                                format!(
                                    "builtin 'cross' expects (Vec3, Vec3), got ({:?}, {:?}).",
                                    a, b
                                ),
                            ));
                        }
                        Ok(ValueType::Vec3)
                    }
                    Builtin::Color | Builtin::ColorHex => Ok(ValueType::Color),
                    Builtin::Rect => Ok(ValueType::Rect),
                    Builtin::Canvas => Ok(ValueType::Canvas),
                };
            }
            if let Some((base, method)) = name.split_once('.') {
                // Distinguish struct method calls (`obj.method`) from qualified function
                // calls (`module.symbol`). For now module-qualified calls resolve to
                // plain function names during semantic/codegen fallback.
                let base_is_receiver = base == "my" || scope.contains_key(base);
                if base_is_receiver {
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

                // `base` isn't a local receiver: treat dotted name as module qualification.
                let qualified = format!("{}.{}", base, method);
                if let Some(resolved_name) = resolve_function_name(&qualified, scope, functions) {
                    let sig = functions
                        .get(resolved_name)
                        .expect("resolved function must exist");
                    validate_call_args(
                        resolved_name,
                        args,
                        sig,
                        scope,
                        memory_state,
                        functions,
                        structs,
                        fn_ctx,
                    )?;
                    return Ok(sig.return_type.clone().unwrap_or(ValueType::Unknown));
                }
                return Err(sem_err(
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in expression call.", name),
                ));
            }
            let Some(resolved_name) = resolve_function_name(name, scope, functions) else {
                return Err(sem_err(
                    SEM_UNKNOWN_FUNCTION,
                    format!("unknown function '{}' in expression call.", name),
                ));
            };
            let sig = functions
                .get(resolved_name)
                .expect("resolved function must exist");
            validate_call_args(
                resolved_name,
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
            if args
                .iter()
                .any(|arg| matches!(arg, Expression::DirectBorrow(_) | Expression::ViewBorrow(_)))
            {
                return Err(sem_err(
                    SEM_TASK_RULE,
                    format!(
                        "call-scoped borrow cannot cross the task boundary in run '{}'.",
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
        Expression::WaitTask { task_name, timeout } => {
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
            if let Some(timeout) = timeout {
                let timeout_ty = infer_expression_type(
                    timeout,
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx,
                )?;
                if timeout_ty != ValueType::Duration {
                    return Err(sem_err(
                        SEM_TASK_RULE,
                        format!("timed wait expects Duration, got {:?}.", timeout_ty),
                    ));
                }
                return Err(sem_err(
                    SEM_TASK_RULE,
                    "timed wait requires an 'on error' handler.".to_string(),
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
        Expression::TimedOut => {
            if fn_ctx.map(|ctx| ctx.is_timed_error_context) != Some(true) {
                return Err(sem_err(
                    SEM_CHANNEL_RULE,
                    "timed_out is only available inside the 'on error' handler of a timed Channel operation or timed Task wait."
                        .to_string(),
                ));
            }
            Ok(ValueType::Bool)
        }
        Expression::BinaryOp { op, left, right } => {
            if op == "neg" {
                let operand = right.as_deref().unwrap_or(left);
                let lt = infer_expression_type(
                    operand,
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx,
                )?;
                if lt == ValueType::Int
                    || lt == ValueType::Float
                    || lt == ValueType::Angle
                    || vector_dimension(&lt).is_some()
                {
                    return Ok(lt);
                }
                return Err(sem_err(
                    SEM_TYPE_MISMATCH,
                    "unary '-' requires numeric operand.".to_string(),
                ));
            }
            if op == "not" {
                let operand = right.as_deref().unwrap_or(left);
                let lt = infer_expression_type(
                    operand,
                    scope,
                    memory_state,
                    functions,
                    structs,
                    fn_ctx,
                )?;
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
            if matches!(lt, ValueType::Time | ValueType::Duration)
                || matches!(rt, ValueType::Time | ValueType::Duration)
            {
                let result = match (op.as_str(), &lt, &rt) {
                    ("+", ValueType::Duration, ValueType::Duration) => Some(ValueType::Duration),
                    ("+", ValueType::Time, ValueType::Duration)
                    | ("+", ValueType::Duration, ValueType::Time) => Some(ValueType::Time),
                    ("-", ValueType::Duration, ValueType::Duration)
                    | ("-", ValueType::Time, ValueType::Time) => Some(ValueType::Duration),
                    ("-", ValueType::Time, ValueType::Duration) => Some(ValueType::Time),
                    (
                        "==" | "!=" | "<" | ">" | "<=" | ">=",
                        ValueType::Duration,
                        ValueType::Duration,
                    )
                    | ("==" | "!=" | "<" | ">" | "<=" | ">=", ValueType::Time, ValueType::Time) => {
                        Some(ValueType::Bool)
                    }
                    _ => None,
                };
                return result.ok_or_else(|| {
                    sem_err(
                        SEM_TYPE_MISMATCH,
                        format!(
                            "operator '{}' is not defined for {:?} and {:?}.",
                            op, lt, rt
                        ),
                    )
                });
            }
            if lt == ValueType::ByteSize || rt == ValueType::ByteSize {
                let result = match (op.as_str(), &lt, &rt) {
                    ("+" | "-", ValueType::ByteSize, ValueType::ByteSize) => {
                        Some(ValueType::ByteSize)
                    }
                    (
                        "==" | "!=" | "<" | ">" | "<=" | ">=",
                        ValueType::ByteSize,
                        ValueType::ByteSize,
                    ) => Some(ValueType::Bool),
                    _ => None,
                };
                return result.ok_or_else(|| {
                    sem_err(
                        SEM_TYPE_MISMATCH,
                        format!(
                            "operator '{}' is not defined for {:?} and {:?}.",
                            op, lt, rt
                        ),
                    )
                });
            }
            if lt == ValueType::Angle || rt == ValueType::Angle {
                let scalar = |ty: &ValueType| matches!(ty, ValueType::Int | ValueType::Float);
                let result = match (op.as_str(), &lt, &rt) {
                    ("+" | "-", ValueType::Angle, ValueType::Angle) => Some(ValueType::Angle),
                    ("*", ValueType::Angle, ty) if scalar(ty) => Some(ValueType::Angle),
                    ("*", ty, ValueType::Angle) if scalar(ty) => Some(ValueType::Angle),
                    ("/", ValueType::Angle, ty) if scalar(ty) => Some(ValueType::Angle),
                    ("/", ValueType::Angle, ValueType::Angle) => Some(ValueType::Float),
                    ("==" | "!=" | "<" | ">" | "<=" | ">=", ValueType::Angle, ValueType::Angle) => {
                        Some(ValueType::Bool)
                    }
                    _ => None,
                };
                return result.ok_or_else(|| {
                    sem_err(
                        SEM_TYPE_MISMATCH,
                        format!(
                            "operator '{}' is not defined for {:?} and {:?}.",
                            op, lt, rt
                        ),
                    )
                });
            }
            if vector_dimension(&lt).is_some() || vector_dimension(&rt).is_some() {
                let scalar = |ty: &ValueType| matches!(ty, ValueType::Int | ValueType::Float);
                let result = match (op.as_str(), &lt, &rt) {
                    ("+" | "-", left, right)
                        if vector_dimension(left).is_some() && left == right =>
                    {
                        Some(left.clone())
                    }
                    ("*", vector, value) if vector_dimension(vector).is_some() && scalar(value) => {
                        Some(vector.clone())
                    }
                    ("*", value, vector) if scalar(value) && vector_dimension(vector).is_some() => {
                        Some(vector.clone())
                    }
                    ("/", vector, value) if vector_dimension(vector).is_some() && scalar(value) => {
                        Some(vector.clone())
                    }
                    _ => None,
                };
                return result.ok_or_else(|| {
                    sem_err(
                        SEM_TYPE_MISMATCH,
                        format!(
                            "operator '{}' is not defined for {:?} and {:?}.",
                            op, lt, rt
                        ),
                    )
                });
            }
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
        Expression::VariableReference(name)
        | Expression::DirectBorrow(name)
        | Expression::ViewBorrow(name)
        | Expression::Move(name) => Ok(memory_state.variable_memory.get(name).cloned()),
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
        Expression::WaitTask { .. } => Ok(None),
        Expression::RunTask { .. } | Expression::Stopping | Expression::TimedOut => Ok(None),
        Expression::LiteralString(_)
        | Expression::ListLiteral(_)
        | Expression::StructConstruction { .. } => Ok(memory_state.active_memory.clone()),
        Expression::BinaryOp { .. } => Ok(None),
        Expression::LiteralInt(_)
        | Expression::LiteralFloat(_)
        | Expression::LiteralBool(_)
        | Expression::LiteralChar(_)
        | Expression::LiteralDuration { .. }
        | Expression::LiteralByteSize { .. }
        | Expression::LiteralAngle { .. } => Ok(None),
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
        Expression::WaitTask { task_name, timeout } => {
            task_name == name
                || timeout
                    .as_deref()
                    .map(|timeout| contains_variable(timeout, name))
                    .unwrap_or(false)
        }
        Expression::Index { base, index } => {
            contains_variable(base, name) || contains_variable(index, name)
        }
        Expression::ListLiteral(items) => items.iter().any(|a| contains_variable(a, name)),
        Expression::LiteralString(_) => false,
        _ => false,
    }
}
