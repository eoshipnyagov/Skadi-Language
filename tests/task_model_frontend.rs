use v01::ast_nodes::{Expression, Statement};
use v01::codegen::ensure_codegen_supported;
use v01::formatter::format_source;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

fn parse_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex task model source");
    parse_program(&tokens).expect("parse task model source")
}

fn semantic_ok(source: &str) {
    let program = parse_ok(source);
    semantic_analyze(&program).expect("semantic task model source");
}

fn semantic_err(source: &str) -> String {
    let program = parse_ok(source);
    semantic_analyze(&program).expect_err("expected semantic error")
}

#[test]
fn parser_accepts_task_and_channel_surface() {
    let program = parse_ok(
        r#"
fn worker() {
    pass
}

fn load_text() Text {
    return "ok"
}

Task worker_task = run worker()
Task(Text) load_task = run load_text()
Channel(Text) events = channel(32)
wait worker_task
new Text loaded_text = wait load_task
events.send(loaded_text)
new Text event_text = events.receive()
stop worker_task
"#,
    );

    assert!(matches!(
        &program.statements[2],
        Statement::VarDecl {
            declared_type: Some(dt),
            value,
            ..
        } if dt == "Task" && matches!(value.as_ref(), Expression::RunTask { call_name, .. } if call_name == "worker")
    ));
    assert!(matches!(
        &program.statements[3],
        Statement::VarDecl {
            declared_type: Some(dt),
            ..
        } if dt == "Task(Text)"
    ));
    assert!(matches!(
        &program.statements[4],
        Statement::VarDecl {
            declared_type: Some(dt),
            ..
        } if dt == "Channel(Text)"
    ));
    assert!(matches!(
        &program.statements[5],
        Statement::ExpressionStatement { expr, .. }
            if matches!(expr.as_ref(), Expression::WaitTask { task_name } if task_name == "worker_task")
    ));
    assert!(matches!(
        program.statements.last(),
        Some(Statement::StopTask { task_name, .. }) if task_name == "worker_task"
    ));
}

#[test]
fn semantic_accepts_task_lifecycle_and_stopping() {
    semantic_ok(
        r#"
fn worker() {
    while not stopping {
        pass
    }
}

Task worker_task = run worker()
stop worker_task
wait worker_task
"#,
    );
}

#[test]
fn semantic_accepts_result_task_and_value_safe_channel() {
    semantic_ok(
        r#"
struct Event {
    Int id
    Text name
}

fn load_event() Event {
    return {id = 1, name = "ready"}
}

Task(Event) event_task = run load_event()
new Event event_value = wait event_task
Channel(Event) events = channel(4)
events.send(event_value)
new Event received_event = events.receive()
"#,
    );
}

#[test]
fn semantic_rejects_wait_and_stop_on_non_task() {
    let wait_err = semantic_err(
        r#"
new Int value = 1
wait value
"#,
    );
    assert!(wait_err.contains("wait expects Task handle"), "{wait_err}");

    let stop_err = semantic_err(
        r#"
new Int value = 1
stop value
"#,
    );
    assert!(stop_err.contains("stop expects Task handle"), "{stop_err}");
}

#[test]
fn semantic_rejects_task_as_regular_value() {
    let return_err = semantic_err(
        r#"
fn worker() {
    pass
}

fn leak_task() Task {
    Task worker_task = run worker()
    return worker_task
}
"#,
    );
    assert!(return_err.contains("Task"), "{return_err}");

    let list_err = semantic_err(
        r#"
new Task List tasks = []
"#,
    );
    assert!(list_err.contains("Task"), "{list_err}");
}

#[test]
fn semantic_rejects_bare_task_for_result_function() {
    let err = semantic_err(
        r#"
fn load_text() Text {
    return "ok"
}

Task load_task = run load_text()
"#,
    );
    assert!(err.contains("type mismatch"), "{err}");
}

#[test]
fn semantic_rejects_repeated_wait_and_stopping_outside_task_context() {
    let repeated_wait = semantic_err(
        r#"
fn worker() {
    pass
}

Task worker_task = run worker()
wait worker_task
wait worker_task
"#,
    );
    assert!(repeated_wait.contains("already waited"), "{repeated_wait}");

    let stopping_err = semantic_err(
        r#"
fn ordinary() Bool {
    return stopping
}
"#,
    );
    assert!(stopping_err.contains("stopping"), "{stopping_err}");
}

#[test]
fn semantic_rejects_channel_handle_messages_and_local_region_payload() {
    let channel_of_memory = semantic_err(
        r#"
Channel(Memory) memories = channel(2)
"#,
    );
    assert!(
        channel_of_memory.contains("value-safe"),
        "{channel_of_memory}"
    );

    let local_region_send = semantic_err(
        r#"
Memory scratch_memory = memory(4kb)
Channel(Text) texts = channel(2)
place in scratch_memory {
    new Text text_value = "owned"
    texts.send(text_value)
}
"#,
    );
    assert!(
        local_region_send.contains("cannot send region-owned value"),
        "{local_region_send}"
    );
}

#[test]
fn style_warning_reports_ignored_run_handle() {
    let program = parse_ok(
        r#"
fn worker() {
    pass
}

run worker()
"#,
    );
    semantic_analyze(&program).expect("ignored run remains semantic-ok");
    let warnings = semantic_style_warnings(&program);
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("task handle ignored")),
        "expected ignored run warning, got {warnings:?}"
    );
}

#[test]
fn backend_gate_rejects_accepted_task_frontend_program() {
    let program = parse_ok(
        r#"
fn worker() {
    pass
}

Task worker_task = run worker()
wait worker_task
"#,
    );
    semantic_analyze(&program).expect("task frontend should be accepted");
    let err = ensure_codegen_supported(&program).expect_err("backend gate should reject task MVP");
    assert!(err.contains("SC-CG-301"), "{err}");
}

#[test]
fn formatter_prints_canonical_task_channel_surface() {
    let formatted = format_source(
        r#"
fn load_text() Text { return "ok" }
Task(Text) load_task = run load_text()
new Text loaded_text = wait load_task
Channel(Text) events = channel(8)
events.send(loaded_text)
stop load_task
"#,
    )
    .expect("format task source");

    assert!(formatted.contains("Task(Text) load_task = run load_text()"));
    assert!(formatted.contains("new Text loaded_text = wait load_task"));
    assert!(formatted.contains("Channel(Text) events = channel(8)"));
    assert!(formatted.contains("events.send(loaded_text)"));
    assert!(formatted.contains("stop load_task"));
}
