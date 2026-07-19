use v01::ast_nodes::{Expression, Statement};
use v01::codegen::ensure_codegen_supported;
use v01::formatter::format_source;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

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

    let mutable_list_payload = semantic_err(
        r#"
Channel(Int List) batches = channel(2)
"#,
    );
    assert!(
        mutable_list_payload.contains("value-safe"),
        "{mutable_list_payload}"
    );

    let loop_owner = semantic_err(
        r#"
loop {
    Channel(Int) values = channel(1)
    break
}
"#,
    );
    assert!(loop_owner.contains("inside a loop"), "{loop_owner}");

    let place_owner = semantic_err(
        r#"
Memory scratch_memory = memory(4kb)
place in scratch_memory {
    Channel(Int) values = channel(1)
}
"#,
    );
    assert!(place_owner.contains("inside 'place in'"), "{place_owner}");
}

#[test]
fn semantic_rejects_ignored_run_handle() {
    let program = parse_ok(
        r#"
fn worker() {
    pass
}

run worker()
"#,
    );
    let err = semantic_analyze(&program).expect_err("ignored run must be rejected");
    assert!(err.contains("SC-SEM-070"), "{err}");
    assert!(err.contains("task handle ignored"), "{err}");
}

#[test]
fn semantic_requires_wait_before_task_owner_scope_ends() {
    let err = semantic_err(
        r#"
fn worker() {
    pass
}

fn launch() {
    Task worker_task = run worker()
}
"#,
    );
    assert!(err.contains("SC-SEM-070"), "{err}");
    assert!(err.contains("must be waited on all paths"), "{err}");
}

#[test]
fn semantic_accepts_wait_on_every_if_path() {
    semantic_ok(
        r#"
fn worker() {
    pass
}

fn launch(Bool use_fast_path) {
    Task worker_task = run worker()
    if use_fast_path {
        wait worker_task
    } else {
        wait worker_task
    }
}
"#,
    );
}

#[test]
fn semantic_rejects_wait_on_only_one_if_path() {
    let err = semantic_err(
        r#"
fn worker() {
    pass
}

fn launch(Bool use_fast_path) {
    Task worker_task = run worker()
    if use_fast_path {
        wait worker_task
    }
}
"#,
    );
    assert!(err.contains("must be waited on all paths"), "{err}");
}

#[test]
fn semantic_rejects_return_and_loop_dependent_task_cleanup() {
    let return_err = semantic_err(
        r#"
fn worker() {
    pass
}

fn launch() Int {
    Task worker_task = run worker()
    return 1
}
"#,
    );
    assert!(
        return_err.contains("must be waited on all paths"),
        "{return_err}"
    );

    let loop_err = semantic_err(
        r#"
fn worker() {
    pass
}

fn launch(Bool ready) {
    Task worker_task = run worker()
    while ready {
        wait worker_task
    }
}
"#,
    );
    assert!(loop_err.contains("cannot depend on a loop"), "{loop_err}");

    let break_err = semantic_err(
        r#"
fn worker() {
    pass
}

fn launch(Bool ready) {
    Task worker_task = run worker()
    while ready {
        wait worker_task
        break
    }
    wait worker_task
}
"#,
    );
    assert!(break_err.contains("cannot depend on a loop"), "{break_err}");
}

#[test]
fn semantic_accepts_complete_task_lifecycle_inside_each_loop_iteration() {
    semantic_ok(include_str!("../examples/concurrency/02_restart_task.skd"));
}

#[test]
fn semantic_rejects_repeated_stop() {
    let err = semantic_err(
        r#"
fn worker() {
    pass
}

Task worker_task = run worker()
stop worker_task
stop worker_task
wait worker_task
"#,
    );
    assert!(err.contains("already stopped"), "{err}");
}

#[test]
fn semantic_rejects_danger_task_entry() {
    let err = semantic_err(
        r#"
danger fn risky_worker() Int {
    return 1
}

Task(Int) worker_task = run risky_worker()
wait worker_task
"#,
    );
    assert!(err.contains("danger fn 'risky_worker'"), "{err}");
}

#[test]
fn semantic_rejects_task_unsafe_arguments_and_results() {
    let memory_err = semantic_err(
        r#"
fn worker(Memory assets_memory) {
    pass
}

Memory assets_memory = memory(4kb)
Task worker_task = run worker(assets_memory)
wait worker_task
"#,
    );
    assert!(memory_err.contains("task-unsafe argument"), "{memory_err}");

    let region_err = semantic_err(
        r#"
fn worker(Text message) {
    output(message)
}

Memory scratch_memory = memory(4kb)
place in scratch_memory {
    new Text message = "owned"
    Task worker_task = run worker(message)
    wait worker_task
}
"#,
    );
    assert!(region_err.contains("region-owned value"), "{region_err}");

    let list_err = semantic_err(
        r#"
fn worker(Int List values) {
    pass
}

new Int List values = [1, 2]
Task worker_task = run worker(values)
wait worker_task
"#,
    );
    assert!(list_err.contains("task-unsafe argument"), "{list_err}");
}

#[test]
fn backend_accepts_void_run_wait_slice_and_emits_runtime_shape() {
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
    ensure_codegen_supported(&program).expect("void run/wait slice should be supported");
    let generated = v01::codegen::transpile_program_to_c(&program);
    assert!(generated.contains("struct SkTask"));
    assert!(generated.contains("sk_task_start(&worker_task"));
    assert!(generated.contains("sk_task_join(&worker_task)"));
    assert!(generated.contains("sk_task_entry_worker"));
    assert!(generated.contains("CreateThread"));
    assert!(generated.contains("pthread_create"));
}

#[test]
fn backend_supports_task_results_cooperative_stop_and_channels() {
    let result_program = parse_ok(
        r#"
fn load_text() Text {
    return "ok"
}

Task(Text) load_task = run load_text()
new Text loaded_text = wait load_task
"#,
    );
    semantic_analyze(&result_program).expect("Task(T) frontend should remain accepted");
    ensure_codegen_supported(&result_program).expect("Task(T) should reach codegen");
    let generated = v01::codegen::transpile_program_to_c(&result_program);
    assert!(generated.contains("const char* result;"));
    assert!(generated.contains("context->result = load_text()"));
    assert!(generated.contains("loaded_text = ((SkTaskContext_load_text*)"));

    let stop_program = parse_ok(
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
    semantic_analyze(&stop_program).expect("stop frontend should remain accepted");
    ensure_codegen_supported(&stop_program).expect("stop/stopping should reach codegen");
    let generated = v01::codegen::transpile_program_to_c(&stop_program);
    assert!(
        generated.contains("sk_task_request_stop(&worker_task)"),
        "{generated}"
    );
    assert!(
        generated.contains("while ((!sk_task_is_stopping()))"),
        "{generated}"
    );
    assert!(generated.contains("InterlockedExchange"), "{generated}");
    assert!(generated.contains("pthread_mutex_lock"), "{generated}");
    assert!(
        generated.contains("static SK_THREAD_LOCAL SkTask *sk_current_task"),
        "{generated}"
    );

    let channel_program = parse_ok(
        r#"
Channel(Text) events = channel(4)
events.send("ready")
new Text event = events.receive()
"#,
    );
    semantic_analyze(&channel_program).expect("Channel frontend should remain accepted");
    ensure_codegen_supported(&channel_program).expect("Channel should reach codegen");
    let generated = v01::codegen::transpile_program_to_c(&channel_program);
    assert!(generated.contains("typedef struct {\n    unsigned char *buffer;"));
    assert!(generated.contains("SkChannel *events = sk_channel_create(4, sizeof(const char*))"));
    assert!(
        generated.contains("sk_channel_send_Text(events, \"ready\")"),
        "{generated}"
    );
    assert!(
        generated.contains("sk_channel_receive_Text(events)"),
        "{generated}"
    );
    assert!(generated.contains("SleepConditionVariableCS"));
    assert!(generated.contains("pthread_cond_wait"));
    assert!(generated.contains("sk_channel_destroy(events)"));
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
