use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use v01::codegen::{ensure_codegen_supported, transpile_program_to_c};
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

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
    let mut c_path = std::env::temp_dir();
    c_path.push(format!("skadi_task_runtime_{stamp}.c"));
    let mut exe_path = std::env::temp_dir();
    exe_path.push(format!("skadi_task_runtime_{stamp}"));
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

fn load_reading(Int sensor_id) Reading {
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

fn sum_numbers(Channel(Int) values, Int count) Int {
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
