use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use v01::codegen::{ensure_codegen_supported, transpile_program_to_c};
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

static NEXT_TEMP_ARTIFACT: AtomicU64 = AtomicU64::new(0);

fn find_c_compiler() -> Option<&'static str> {
    let candidates: &[&str] = if cfg!(windows) {
        &["gcc", "clang", "cc"]
    } else {
        &["clang", "gcc", "cc"]
    };
    candidates
        .iter()
        .find(|&&compiler| Command::new(compiler).arg("--version").output().is_ok())
        .copied()
}

fn compile_and_run(compiler: &str, c_source: &str) -> std::process::Output {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let sequence = NEXT_TEMP_ARTIFACT.fetch_add(1, Ordering::Relaxed);
    let artifact_id = format!("{}_{stamp}_{sequence}", std::process::id());
    let mut c_path = std::env::temp_dir();
    c_path.push(format!("skadi_task_runtime_{artifact_id}.c"));
    let mut exe_path = std::env::temp_dir();
    exe_path.push(format!("skadi_task_runtime_{artifact_id}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }
    fs::write(&c_path, c_source).expect("write generated C");

    let mut compile = Command::new(compiler);
    compile.arg(&c_path).arg("-o").arg(&exe_path).arg("-lm");
    if !cfg!(windows) {
        compile.arg("-pthread");
    }
    let compiled = compile.output().expect("run C compiler");
    assert!(
        compiled.status.success(),
        "task runtime C compile failed: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let output = Command::new(&exe_path).output().expect("run task binary");
    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
    output
}

#[test]
fn void_tasks_run_and_wait_end_to_end() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Task runtime e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
fn worker(Int worker_id, Text message) {
    output(worker_id)
    output(message)
}

Task first_task = run worker(11, "first")
Task second_task = run worker(22, "second")
wait first_task
wait second_task
output("joined")
"#;
    let tokens = lex(source).expect("lex task runtime source");
    let program = parse_program(&tokens).expect("parse task runtime source");
    semantic_analyze(&program).expect("semantic task runtime source");
    ensure_codegen_supported(&program).expect("void task slice should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "task runtime binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("11"), "{stdout}");
    assert!(stdout.contains("22"), "{stdout}");
    assert!(stdout.contains("first"), "{stdout}");
    assert!(stdout.contains("second"), "{stdout}");
    assert!(stdout.contains("joined"), "{stdout}");
}

#[test]
fn timed_task_wait_preserves_handle_on_timeout_and_consumes_it_on_success() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping timed Task e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = include_str!("../examples/concurrency/05_timed_task_wait.skd");
    let tokens = lex(source).expect("lex timed Task source");
    let program = parse_program(&tokens).expect("parse timed Task source");
    semantic_analyze(&program).expect("semantic timed Task source");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "timed Task binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let lines = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(lines, ["slow task exceeded its budget", "41", "7"]);
}

#[test]
fn zero_duration_task_wait_is_an_immediate_completion_check() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping zero-duration Task e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
fn slow_value() returns Int {
    sleep(30ms)
    return 41
}

fn ready_value() returns Int {
    return 7
}

Task(Int) slow_task = run slow_value()
new Int slow_result = 0
slow_result = wait slow_task for 0ms on error {
    if timed_out {
        output("pending")
    }
    stop slow_task
    slow_result = wait slow_task
}
output(slow_result)

Task(Int) ready_task = run ready_value()
sleep(50ms)
new Int ready_result = 0
ready_result = wait ready_task for 0ms on error {
    output("unexpected timeout")
    ready_result = wait ready_task
}
output(ready_result)
"#;
    let tokens = lex(source).expect("lex zero-duration Task source");
    let program = parse_program(&tokens).expect("parse zero-duration Task source");
    semantic_analyze(&program).expect("semantic zero-duration Task source");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "zero-duration Task binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let lines = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(lines, ["pending", "41", "7"]);
}

#[test]
fn timed_channel_operations_distinguish_timeout_close_and_success() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping timed Channel e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
Channel(Int) values = channel(1)
new Int value = 0

value = values.receive_for(20ms) on error {
    if timed_out {
        output(101)
    }
}

values.send(7)
value = values.receive_for(0ms) on error {
    output(-1)
}
output(value)

values.send(11)
values.send_for(22, 20ms) on error {
    if timed_out {
        output(102)
    }
}
output(values.receive())

values.close()
value = values.receive_for(1ms) on error {
    if timed_out {
        output(-2)
    } else {
        output(103)
    }
}
"#;
    let tokens = lex(source).expect("lex timed Channel source");
    let program = parse_program(&tokens).expect("parse timed Channel source");
    semantic_analyze(&program).expect("semantic timed Channel source");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "timed Channel binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let lines = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(lines, ["101", "7", "102", "11", "103"]);
}

#[test]
fn stop_wins_over_long_channel_timeout() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping timed cancellation e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
fn wait_for_value(Channel(Int) values, Channel(Int) ready) returns Int {
    ready.send(1)
    new Int value = 0
    value = values.receive_for(5s) on error {
        if stopping {
            return 201
        }
        if timed_out {
            return 202
        }
        return 203
    }
    return value
}

Channel(Int) values = channel(1)
Channel(Int) ready = channel(1)
Task(Int) worker = run wait_for_value(values, ready)
new Int signal = ready.receive()
stop worker
new Int result = wait worker
output(result)
"#;
    let tokens = lex(source).expect("lex timed cancellation source");
    let program = parse_program(&tokens).expect("parse timed cancellation source");
    semantic_analyze(&program).expect("semantic timed cancellation source");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "timed cancellation binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "201");
}

#[test]
fn close_wakes_timed_receive_as_close_not_timeout() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping timed close e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
fn wait_for_close(Channel(Int) values, Channel(Int) ready) returns Int {
    ready.send(1)
    new Int value = 0
    value = values.receive_for(5s) on error {
        if timed_out {
            return 302
        }
        return 301
    }
    return value
}

Channel(Int) values = channel(1)
Channel(Int) ready = channel(1)
Task(Int) worker = run wait_for_close(values, ready)
new Int signal = ready.receive()
values.close()
new Int result = wait worker
output(result)
"#;
    let tokens = lex(source).expect("lex timed close source");
    let program = parse_program(&tokens).expect("parse timed close source");
    semantic_analyze(&program).expect("semantic timed close source");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "timed close binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "301");
}

#[test]
fn task_results_move_to_waiting_scope_end_to_end() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Task(T) runtime e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
struct Reading {
    Int sensor_id
    Float value
}

fn calculate(Int base) Int {
    return base + 7
}

fn load_reading(Int sensor_id) returns Reading {
    return {sensor_id = sensor_id, value = 21.5}
}

fn status_text() Text {
    return "ready"
}

Task(Int) number_task = run calculate(35)
Task(Reading) reading_task = run load_reading(9)
Task(Text) text_task = run status_text()
new Int answer = wait number_task
new Reading reading = wait reading_task
new Text status = wait text_task
output(answer)
output(reading.sensor_id)
output(status)
"#;
    let tokens = lex(source).expect("lex Task(T) runtime source");
    let program = parse_program(&tokens).expect("parse Task(T) runtime source");
    semantic_analyze(&program).expect("semantic Task(T) runtime source");
    ensure_codegen_supported(&program).expect("Task(T) should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "Task(T) runtime binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("42"), "{stdout}");
    assert!(stdout.contains("9"), "{stdout}");
    assert!(stdout.contains("ready"), "{stdout}");
}

#[test]
fn task_stop_is_observed_and_result_remains_joinable_end_to_end() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Task stop runtime e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
fn work_until_stopped() Int {
    while not stopping {
        pass
    }
    return 73
}

Task(Int) worker_task = run work_until_stopped()
stop worker_task
new Int result = wait worker_task
output(result)
output("stopped and joined")
"#;
    let tokens = lex(source).expect("lex Task stop runtime source");
    let program = parse_program(&tokens).expect("parse Task stop runtime source");
    semantic_analyze(&program).expect("semantic Task stop runtime source");
    ensure_codegen_supported(&program).expect("Task stop should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "Task stop runtime binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("73"), "{stdout}");
    assert!(stdout.contains("stopped and joined"), "{stdout}");
}

#[test]
fn stop_cancels_blocked_receive_without_closing_or_draining_channel() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping blocked receive cancellation e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = include_str!("../examples/concurrency/03_cancel_blocked_channel.skd");
    let tokens = lex(source).expect("lex blocked receive cancellation source");
    let program = parse_program(&tokens).expect("parse blocked receive cancellation source");
    semantic_analyze(&program).expect("semantic blocked receive cancellation source");
    ensure_codegen_supported(&program).expect("blocked receive cancellation should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "blocked receive cancellation binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["1", "41", "7"]
    );
}

#[test]
fn stop_cancels_blocked_send_without_closing_or_mutating_channel() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping blocked send cancellation e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
fn send_value(Channel(Int) values, Channel(Int) ready) returns Int {
    ready.send(1)
    values.send(20) on error {
        if stopping {
            return 51
        }
        return 52
    }
    return 0
}

Channel(Int) values = channel(1)
Channel(Int) ready = channel(1)
values.send(10)
Task(Int) worker_task = run send_value(values, ready)
new Int signal = ready.receive()
stop worker_task
new Int result = wait worker_task

new Int original = values.receive()
values.send(30)
new Int next = values.receive()
output(signal)
output(result)
output(original)
output(next)
"#;
    let tokens = lex(source).expect("lex blocked send cancellation source");
    let program = parse_program(&tokens).expect("parse blocked send cancellation source");
    semantic_analyze(&program).expect("semantic blocked send cancellation source");
    ensure_codegen_supported(&program).expect("blocked send cancellation should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "blocked send cancellation binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["1", "51", "10", "30"]
    );
}

#[test]
fn bounded_channels_preserve_fifo_and_backpressure_end_to_end() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Channel runtime e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
struct Event {
    Int id
    Text name
}

fn produce(Channel(Event) events) {
    new Event first = {id = 10, name = "first"}
    new Event second = {id = 20, name = "second"}
    events.send(first)
    events.send(second)
}

fn consume(Channel(Int) numbers) Int {
    new Int first = numbers.receive()
    new Int second = numbers.receive()
    return first * 10 + second
}

Channel(Event) events = channel(1)
Channel(Int) numbers = channel(1)
Task producer_task = run produce(events)
Task(Int) consumer_task = run consume(numbers)
numbers.send(4)
numbers.send(2)
new Event first_event = events.receive()
new Event second_event = events.receive()
wait producer_task
new Int answer = wait consumer_task
new Text first_name = first_event.name
new Text second_name = second_event.name
output(first_event.id)
output(first_name)
output(second_event.id)
output(second_name)
output(answer)
"#;
    let tokens = lex(source).expect("lex Channel runtime source");
    let program = parse_program(&tokens).expect("parse Channel runtime source");
    semantic_analyze(&program).expect("semantic Channel runtime source");
    ensure_codegen_supported(&program).expect("Channel should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "Channel runtime binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, ["10", "first", "20", "second", "42"], "{stdout}");
}

#[test]
fn invalid_channel_capacity_has_stable_runtime_diagnostic() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Channel diagnostic e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
Channel(Int) values = channel(0)
"#;
    let tokens = lex(source).expect("lex invalid Channel source");
    let program = parse_program(&tokens).expect("parse invalid Channel source");
    semantic_analyze(&program).expect("capacity value is a runtime contract");
    ensure_codegen_supported(&program).expect("Channel should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        !run.status.success(),
        "invalid capacity unexpectedly succeeded"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("SC-RT-312"), "{stderr}");
    assert!(stderr.contains("capacity"), "{stderr}");
}

#[test]
fn function_local_channel_is_destroyed_after_return_value_is_copied() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping local Channel e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
fn local_roundtrip() Int {
    Channel(Int) values = channel(1)
    values.send(91)
    return values.receive()
}

new Int result = local_roundtrip()
output(result)
"#;
    let tokens = lex(source).expect("lex local Channel source");
    let program = parse_program(&tokens).expect("parse local Channel source");
    semantic_analyze(&program).expect("semantic local Channel source");
    ensure_codegen_supported(&program).expect("Channel should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "local Channel binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "91");
}

#[test]
fn bounded_channel_repeated_producer_consumer_stress() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Channel stress e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
fn produce_numbers(Channel(Int) values, Int count) {
    new Int index = 0
    while index < count {
        values.send(index)
        index++
    }
}

fn sum_numbers(Channel(Int) values, Int count) returns Int {
    new Int total = 0
    new Int index = 0
    while index < count {
        new Int value = values.receive()
        total = total + value
        index++
    }
    return total
}

Channel(Int) values = channel(4)
Task producer_task = run produce_numbers(values, 1000)
Task(Int) sum_task = run sum_numbers(values, 1000)
wait producer_task
new Int total = wait sum_task
output(total)
"#;
    let tokens = lex(source).expect("lex Channel stress source");
    let program = parse_program(&tokens).expect("parse Channel stress source");
    semantic_analyze(&program).expect("semantic Channel stress source");
    ensure_codegen_supported(&program).expect("Channel should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "Channel stress binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "499500");
}

#[test]
fn five_tasks_run_concurrently_and_share_a_bounded_channel() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping five-task runtime e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = include_str!("../examples/concurrency/01_five_workers.skd");
    let tokens = lex(source).expect("lex five-task source");
    let program = parse_program(&tokens).expect("parse five-task source");
    semantic_analyze(&program).expect("semantic five-task source");
    ensure_codegen_supported(&program).expect("five tasks should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "five-task binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "55");
}

#[test]
fn task_function_can_be_restarted_with_a_fresh_handle_in_a_loop() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping task restart e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = include_str!("../examples/concurrency/02_restart_task.skd");
    let tokens = lex(source).expect("lex task restart source");
    let program = parse_program(&tokens).expect("parse task restart source");
    semantic_analyze(&program).expect("semantic task restart source");
    ensure_codegen_supported(&program).expect("task restart should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "task restart binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "15");
}

#[test]
fn closed_channel_drains_and_reports_fallible_operations_end_to_end() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Channel close e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
Channel(Int) values = channel(2)
values.send(7)
values.close()

new Int first = 0
first = values.receive() on error {
    output("unexpected")
}

output(first)

new Int second = 0
second = values.receive() on error {
    output("drained")
}

values.send(9) on error {
    output("closed")
}

new Bool accepted = values.try_send(11)
output(accepted)
"#;
    let tokens = lex(source).expect("lex Channel close source");
    let program = parse_program(&tokens).expect("parse Channel close source");
    semantic_analyze(&program).expect("semantic Channel close source");
    ensure_codegen_supported(&program).expect("Channel close should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "Channel close binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["7", "drained", "closed", "false"]
    );
}

#[test]
fn closed_channel_operations_enter_on_error_handlers_end_to_end() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping closed Channel handler e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
Channel(Int) values = channel(2)
values.close()

values.send(7) on error {
    output("closed-send")
}

values.close() on error {
    output("closed-close")
}
"#;
    let tokens = lex(source).expect("lex closed Channel handler source");
    let program = parse_program(&tokens).expect("parse closed Channel handler source");
    semantic_analyze(&program).expect("semantic closed Channel handler source");
    ensure_codegen_supported(&program).expect("closed Channel handlers should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "closed Channel handler binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("closed-send"), "{stdout}");
    assert!(stdout.contains("closed-close"), "{stdout}");
}

#[test]
fn periodic_interrupt_bridges_to_normal_context_through_try_send() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Interrupt runtime e2e: no clang/gcc/cc in PATH.");
        return;
    };
    let source = r#"
Channel(Int) ticks = channel(8)
Interrupt timer = interrupts.periodic(5ms)

on interrupt timer {
    ticks.try_send(1)
}

sleep(25ms)
new Int tick = ticks.receive()
output(tick)
"#;
    let tokens = lex(source).expect("lex Interrupt source");
    let program = parse_program(&tokens).expect("parse Interrupt source");
    semantic_analyze(&program).expect("semantic Interrupt source");
    ensure_codegen_supported(&program).expect("Interrupt should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "Interrupt binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "1");
}

#[test]
fn timed_channel_showcase_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping timed Channel showcase: no clang/gcc/cc in PATH.");
        return;
    };
    let source = include_str!("../examples/concurrency/04_timed_channel.skd");
    let tokens = lex(source).expect("lex timed Channel showcase");
    let program = parse_program(&tokens).expect("parse timed Channel showcase");
    semantic_analyze(&program).expect("semantic timed Channel showcase");
    ensure_codegen_supported(&program).expect("timed Channel showcase should reach codegen");
    let generated = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &generated);

    assert!(
        run.status.success(),
        "timed Channel showcase failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.lines().any(|line| line.trim() == "42"), "{stdout}");
    assert!(stdout.contains("no reading before deadline"), "{stdout}");
}
