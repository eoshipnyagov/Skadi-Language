use std::collections::HashSet;

use crate::ast_nodes::{Expression, Location, Program, Statement};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisFactKind {
    BlockingChannelSend,
    BlockingChannelReceive,
    TimedChannelSend,
    TimedChannelReceive,
    TimedTaskWait,
    IncompleteWhen,
    ResourceCreated,
    ResourceBorrowed,
    ResourceMoved,
    ResourceClosed,
    TaskLifecycle,
    MemoryLifecycle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisFactLevel {
    Info,
    Attention,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisFact {
    pub kind: AnalysisFactKind,
    pub level: AnalysisFactLevel,
    pub code: &'static str,
    pub line: u32,
    pub col: u32,
    pub context: String,
    pub subject: Option<String>,
    pub summary: String,
    pub explanation: String,
    pub action: String,
}

pub fn collect_analysis_facts(program: &Program) -> Vec<AnalysisFact> {
    let mut task_entries = HashSet::new();
    collect_task_entries(&program.statements, &mut task_entries);

    let mut facts = Vec::new();
    collect_statement_facts(&program.statements, None, &task_entries, &mut facts);
    facts
}

fn collect_task_entries(statements: &[Statement], entries: &mut HashSet<String>) {
    for statement in statements {
        match statement {
            Statement::VarDecl { value, .. }
            | Statement::Assignment { value, .. }
            | Statement::FieldAssignment { value, .. }
            | Statement::ListPush { value, .. } => collect_task_entries_from_expr(value, entries),
            Statement::FunctionDef { body, .. } => collect_task_entries(&body.statements, entries),
            Statement::StructDecl { methods, .. } => {
                for method in methods {
                    collect_task_entries(&method.body.statements, entries);
                }
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                collect_task_entries_from_expr(condition, entries);
                collect_task_entries(&then_block.statements, entries);
                if let Some(block) = else_block {
                    collect_task_entries(&block.statements, entries);
                }
            }
            Statement::ForLoop {
                initialization,
                condition,
                update,
                body,
                ..
            } => {
                for expression in [initialization, condition, update].into_iter().flatten() {
                    collect_task_entries_from_expr(expression, entries);
                }
                collect_task_entries(&body.statements, entries);
            }
            Statement::WhileLoop {
                condition, body, ..
            } => {
                collect_task_entries_from_expr(condition, entries);
                collect_task_entries(&body.statements, entries);
            }
            Statement::LoopStatement { body, .. } | Statement::OnBlock { body, .. } => {
                collect_task_entries(&body.statements, entries);
            }
            Statement::WhenBlock {
                when_expression,
                cases,
                else_block,
                ..
            } => {
                collect_task_entries_from_expr(when_expression, entries);
                for (expressions, block) in cases {
                    for expression in expressions {
                        collect_task_entries_from_expr(expression, entries);
                    }
                    collect_task_entries(&block.statements, entries);
                }
                if let Some(block) = else_block {
                    collect_task_entries(&block.statements, entries);
                }
            }
            Statement::PlaceIn { body, on_error, .. } => {
                collect_task_entries(&body.statements, entries);
                if let Some(block) = on_error {
                    collect_task_entries(&block.statements, entries);
                }
            }
            Statement::MemoryDecl { on_error, .. } => {
                if let Some(block) = on_error {
                    collect_task_entries(&block.statements, entries);
                }
            }
            Statement::DangerAssignOnError { args, on_error, .. }
            | Statement::DangerCallOnError { args, on_error, .. } => {
                for argument in args {
                    collect_task_entries_from_expr(argument, entries);
                }
                collect_task_entries(&on_error.statements, entries);
            }
            Statement::ListPopOnError { on_error, .. } => {
                collect_task_entries(&on_error.statements, entries);
            }
            Statement::ReturnStatement { value, .. } => {
                if let Some(value) = value {
                    collect_task_entries_from_expr(value, entries);
                }
            }
            Statement::ExpressionStatement { expr, .. } => {
                collect_task_entries_from_expr(expr, entries)
            }
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => {
                collect_task_entries(statements, entries)
            }
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

fn collect_task_entries_from_expr(expression: &Expression, entries: &mut HashSet<String>) {
    match expression {
        Expression::RunTask { call_name, args } => {
            entries.insert(call_name.clone());
            for argument in args {
                collect_task_entries_from_expr(argument, entries);
            }
        }
        Expression::Call { args, .. } | Expression::ListLiteral(args) => {
            for argument in args {
                collect_task_entries_from_expr(argument, entries);
            }
        }
        Expression::Index { base, index } => {
            collect_task_entries_from_expr(base, entries);
            collect_task_entries_from_expr(index, entries);
        }
        Expression::BinaryOp { left, right, .. } => {
            collect_task_entries_from_expr(left, entries);
            if let Some(right) = right {
                collect_task_entries_from_expr(right, entries);
            }
        }
        Expression::StructConstruction { fields } => {
            for value in fields.values() {
                collect_task_entries_from_expr(value, entries);
            }
        }
        Expression::WaitTask { .. }
        | Expression::Stopping
        | Expression::TimedOut
        | Expression::VariableReference(_)
        | Expression::DirectBorrow(_)
        | Expression::ViewBorrow(_)
        | Expression::Move(_)
        | Expression::MemberAccess { .. }
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

fn collect_statement_facts(
    statements: &[Statement],
    function: Option<&str>,
    task_entries: &HashSet<String>,
    facts: &mut Vec<AnalysisFact>,
) {
    for statement in statements {
        match statement {
            Statement::VarDecl {
                name,
                value,
                declared_type,
                loc,
                ..
            } => {
                collect_expression_facts(value, loc, false, function, task_entries, facts);
                if let Some(resource_type) = declared_type
                    && is_lifecycle_resource_type(resource_type)
                {
                    push_lifecycle_fact(
                        facts,
                        AnalysisFactKind::ResourceCreated,
                        "SC-AN-301",
                        loc,
                        function,
                        task_entries,
                        name,
                        format!("'{name}' becomes the owner of {resource_type}"),
                        "the binding owns deterministic cleanup responsibility",
                        "follow this subject to verify its final close, move, wait, or scope cleanup",
                    );
                }
                if matches!(value.as_ref(), Expression::RunTask { .. }) {
                    push_lifecycle_fact(
                        facts,
                        AnalysisFactKind::TaskLifecycle,
                        "SC-AN-311",
                        loc,
                        function,
                        task_entries,
                        name,
                        format!("task '{name}' starts"),
                        "the task handle remains owned by this scope until wait completes",
                        "ensure every reachable path eventually waits this handle",
                    );
                }
            }
            Statement::Assignment { value, loc, .. }
            | Statement::FieldAssignment { value, loc, .. }
            | Statement::ListPush { value, loc, .. } => {
                collect_expression_facts(value, loc, false, function, task_entries, facts)
            }
            Statement::FunctionDef {
                name, body, loc, ..
            } => {
                let _ = loc;
                collect_statement_facts(&body.statements, Some(name), task_entries, facts);
            }
            Statement::StructDecl { methods, .. } => {
                for method in methods {
                    collect_statement_facts(
                        &method.body.statements,
                        Some(&method.name),
                        task_entries,
                        facts,
                    );
                }
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                loc,
            } => {
                collect_expression_facts(condition, loc, false, function, task_entries, facts);
                collect_statement_facts(&then_block.statements, function, task_entries, facts);
                if let Some(block) = else_block {
                    collect_statement_facts(&block.statements, function, task_entries, facts);
                }
            }
            Statement::ForLoop {
                initialization,
                condition,
                update,
                body,
                loc,
                ..
            } => {
                for expression in [initialization, condition, update].into_iter().flatten() {
                    collect_expression_facts(expression, loc, false, function, task_entries, facts);
                }
                collect_statement_facts(&body.statements, function, task_entries, facts);
            }
            Statement::WhileLoop {
                condition,
                body,
                loc,
            } => {
                collect_expression_facts(condition, loc, false, function, task_entries, facts);
                collect_statement_facts(&body.statements, function, task_entries, facts);
            }
            Statement::LoopStatement { body, .. } | Statement::OnBlock { body, .. } => {
                collect_statement_facts(&body.statements, function, task_entries, facts);
            }
            Statement::WhenBlock {
                when_expression,
                cases,
                else_block,
                loc,
            } => {
                collect_expression_facts(
                    when_expression,
                    loc,
                    false,
                    function,
                    task_entries,
                    facts,
                );
                if else_block.is_none() {
                    facts.push(AnalysisFact {
                        kind: AnalysisFactKind::IncompleteWhen,
                        level: AnalysisFactLevel::Attention,
                        code: "SC-AN-201",
                        line: loc.line,
                        col: loc.column,
                        context: context_name(function, task_entries),
                        subject: None,
                        summary: "when has no else branch".to_string(),
                        explanation: "values not covered by an is branch leave the when block without an explicit outcome".to_string(),
                        action: "add an else branch when exhaustive behavior is required".to_string(),
                    });
                }
                for (expressions, block) in cases {
                    for expression in expressions {
                        collect_expression_facts(
                            expression,
                            loc,
                            false,
                            function,
                            task_entries,
                            facts,
                        );
                    }
                    collect_statement_facts(&block.statements, function, task_entries, facts);
                }
                if let Some(block) = else_block {
                    collect_statement_facts(&block.statements, function, task_entries, facts);
                }
            }
            Statement::PlaceIn { body, on_error, .. } => {
                collect_statement_facts(&body.statements, function, task_entries, facts);
                if let Some(block) = on_error {
                    collect_statement_facts(&block.statements, function, task_entries, facts);
                }
            }
            Statement::MemoryDecl {
                name,
                on_error,
                loc,
                ..
            } => {
                push_lifecycle_fact(
                    facts,
                    AnalysisFactKind::MemoryLifecycle,
                    "SC-AN-321",
                    loc,
                    function,
                    task_entries,
                    name,
                    format!("Memory region '{name}' is created"),
                    "values placed in the region share its lifetime boundary",
                    "review clear and scope exit points before values escape",
                );
                if let Some(block) = on_error {
                    collect_statement_facts(&block.statements, function, task_entries, facts);
                }
            }
            Statement::DangerAssignOnError {
                call_name,
                args,
                on_error,
                loc,
                ..
            }
            | Statement::DangerCallOnError {
                call_name,
                args,
                on_error,
                loc,
            } => {
                if call_name == "__task_wait_for"
                    && let [Expression::VariableReference(task_name), _] = args.as_slice()
                {
                    push_lifecycle_fact(
                        facts,
                        AnalysisFactKind::TimedTaskWait,
                        "SC-AN-314",
                        loc,
                        function,
                        task_entries,
                        task_name,
                        format!("task '{task_name}' is waited with a deadline"),
                        "success joins and consumes the handle; timeout enters on error while the handle remains live",
                        "finish the live timeout path with stop/wait or another path-proven ownership action",
                    );
                } else {
                    collect_call_fact(call_name, loc, true, function, task_entries, facts);
                }
                for argument in args {
                    collect_expression_facts(argument, loc, false, function, task_entries, facts);
                }
                collect_statement_facts(&on_error.statements, function, task_entries, facts);
            }
            Statement::ListPopOnError { on_error, .. } => {
                collect_statement_facts(&on_error.statements, function, task_entries, facts)
            }
            Statement::ReturnStatement { value, loc } => {
                if let Some(value) = value {
                    collect_expression_facts(value, loc, false, function, task_entries, facts);
                }
            }
            Statement::ExpressionStatement { expr, loc } => {
                collect_expression_facts(expr, loc, false, function, task_entries, facts)
            }
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => {
                collect_statement_facts(statements, function, task_entries, facts)
            }
            Statement::MemoryClear { memory_name, loc } => push_lifecycle_fact(
                facts,
                AnalysisFactKind::MemoryLifecycle,
                "SC-AN-322",
                loc,
                function,
                task_entries,
                memory_name,
                format!("Memory region '{memory_name}' is cleared"),
                "all region-backed dynamic values become unavailable at this boundary",
                "verify no later path uses or returns values owned by this region",
            ),
            Statement::StopTask { task_name, loc } => push_lifecycle_fact(
                facts,
                AnalysisFactKind::TaskLifecycle,
                "SC-AN-312",
                loc,
                function,
                task_entries,
                task_name,
                format!("stop requested for task '{task_name}'"),
                "stop is cooperative and does not release the task handle",
                "keep the matching wait on every reachable path",
            ),
            Statement::ReturnError { .. }
            | Statement::IncDec { .. }
            | Statement::BreakStatement { .. }
            | Statement::ContinueStatement { .. }
            | Statement::PassStatement { .. }
            | Statement::LabelDecl { .. }
            | Statement::TagDecl { .. } => {}
        }
    }
}

fn collect_expression_facts(
    expression: &Expression,
    loc: &Location,
    handled: bool,
    function: Option<&str>,
    task_entries: &HashSet<String>,
    facts: &mut Vec<AnalysisFact>,
) {
    match expression {
        Expression::Call { name, args } => {
            collect_call_fact(name, loc, handled, function, task_entries, facts);
            for argument in args {
                collect_expression_facts(argument, loc, handled, function, task_entries, facts);
            }
        }
        Expression::RunTask { args, .. } | Expression::ListLiteral(args) => {
            for argument in args {
                collect_expression_facts(argument, loc, handled, function, task_entries, facts);
            }
        }
        Expression::Index { base, index } => {
            collect_expression_facts(base, loc, handled, function, task_entries, facts);
            collect_expression_facts(index, loc, handled, function, task_entries, facts);
        }
        Expression::BinaryOp { left, right, .. } => {
            collect_expression_facts(left, loc, handled, function, task_entries, facts);
            if let Some(right) = right {
                collect_expression_facts(right, loc, handled, function, task_entries, facts);
            }
        }
        Expression::StructConstruction { fields } => {
            for value in fields.values() {
                collect_expression_facts(value, loc, handled, function, task_entries, facts);
            }
        }
        Expression::WaitTask { task_name, .. } => push_lifecycle_fact(
            facts,
            AnalysisFactKind::TaskLifecycle,
            "SC-AN-313",
            loc,
            function,
            task_entries,
            task_name,
            format!("task '{task_name}' is waited and joined"),
            "wait is the ownership boundary that releases the native task lifecycle",
            "keep this wait reachable from every path after task creation",
        ),
        Expression::DirectBorrow(name) => push_lifecycle_fact(
            facts,
            AnalysisFactKind::ResourceBorrowed,
            "SC-AN-302",
            loc,
            function,
            task_entries,
            name,
            format!("'{name}' is borrowed for direct mutation"),
            "ownership stays with the caller while one callee may mutate the value",
            "ensure no competing borrow or owner use overlaps this call",
        ),
        Expression::ViewBorrow(name) => push_lifecycle_fact(
            facts,
            AnalysisFactKind::ResourceBorrowed,
            "SC-AN-302",
            loc,
            function,
            task_entries,
            name,
            format!("'{name}' is borrowed as a read-only view"),
            "ownership stays with the caller and the callee cannot mutate the value",
            "prefer view when the callee only needs to inspect the resource",
        ),
        Expression::Move(name) => push_lifecycle_fact(
            facts,
            AnalysisFactKind::ResourceMoved,
            "SC-AN-303",
            loc,
            function,
            task_entries,
            name,
            format!("ownership of '{name}' moves away from this binding"),
            "the previous binding is unavailable after this source location",
            "continue the lifecycle from the receiving binding or function",
        ),
        Expression::Stopping
        | Expression::TimedOut
        | Expression::VariableReference(_)
        | Expression::MemberAccess { .. }
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

fn collect_call_fact(
    call_name: &str,
    loc: &Location,
    handled: bool,
    function: Option<&str>,
    task_entries: &HashSet<String>,
    facts: &mut Vec<AnalysisFact>,
) {
    let Some((channel, operation)) = call_name.split_once('.') else {
        return;
    };
    if operation == "close" {
        push_lifecycle_fact(
            facts,
            AnalysisFactKind::ResourceClosed,
            "SC-AN-304",
            loc,
            function,
            task_entries,
            channel,
            format!("resource '{channel}' is explicitly closed"),
            "close ends the resource's usable lifecycle before automatic scope cleanup",
            "verify later operations either use on error or are unreachable",
        );
        return;
    }
    let (kind, code) = match operation {
        "send" => (AnalysisFactKind::BlockingChannelSend, "SC-AN-101"),
        "receive" => (AnalysisFactKind::BlockingChannelReceive, "SC-AN-102"),
        "send_for" => (AnalysisFactKind::TimedChannelSend, "SC-AN-103"),
        "receive_for" => (AnalysisFactKind::TimedChannelReceive, "SC-AN-104"),
        _ => return,
    };
    let timed = matches!(operation, "send_for" | "receive_for");
    let task_entry = function.is_some_and(|name| task_entries.contains(name));
    let level = if task_entry && !handled {
        AnalysisFactLevel::Attention
    } else {
        AnalysisFactLevel::Info
    };
    let cancellation = if timed {
        "the wait is bounded by Duration; on error can distinguish stopping, timed_out, and channel close"
    } else if handled {
        "close and task cancellation are handled by the attached on error block"
    } else if task_entry {
        "task cancellation reaches this operation, but no on error handler is attached"
    } else {
        "the operation can wait until channel state changes"
    };
    let action = if timed {
        "handle stopping first, timed_out second, and treat the remaining path as channel close"
    } else if task_entry && !handled {
        "attach on error and use stopping when cancellation needs a separate path"
    } else {
        "verify that the channel protocol always supplies the matching producer or consumer"
    };

    facts.push(AnalysisFact {
        kind,
        level,
        code,
        line: loc.line,
        col: loc.column,
        context: context_name(function, task_entries),
        subject: Some(channel.to_string()),
        summary: format!("'{channel}.{operation}' may block"),
        explanation: cancellation.to_string(),
        action: action.to_string(),
    });
}

fn is_lifecycle_resource_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "Canvas" | "Window" | "Interrupt" | "Task" | "Memory"
    ) || type_name.starts_with("Channel(")
        || type_name.starts_with("Task(")
}

#[allow(clippy::too_many_arguments)]
fn push_lifecycle_fact(
    facts: &mut Vec<AnalysisFact>,
    kind: AnalysisFactKind,
    code: &'static str,
    loc: &Location,
    function: Option<&str>,
    task_entries: &HashSet<String>,
    subject: &str,
    summary: String,
    explanation: &str,
    action: &str,
) {
    facts.push(AnalysisFact {
        kind,
        level: AnalysisFactLevel::Info,
        code,
        line: loc.line,
        col: loc.column,
        context: context_name(function, task_entries),
        subject: Some(subject.to_string()),
        summary,
        explanation: explanation.to_string(),
        action: action.to_string(),
    });
}

fn context_name(function: Option<&str>, task_entries: &HashSet<String>) -> String {
    match function {
        Some(name) if task_entries.contains(name) => format!("task entry '{name}'"),
        Some(name) => format!("function '{name}'"),
        None => "top-level workflow".to_string(),
    }
}
