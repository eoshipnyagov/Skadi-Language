#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use v01::codegen::{ensure_codegen_supported, transpile_program_to_c};
#[cfg(unix)]
use v01::lexer::lex;
#[cfg(unix)]
use v01::parser::parse_program;
#[cfg(unix)]
use v01::semantic_analysis::semantic_analyze;

#[test]
fn channel_runtime_is_clean_under_thread_sanitizer() {
    if std::env::var("SKADI_REQUIRE_TSAN").as_deref() != Ok("1") {
        eprintln!("Skipping required TSan gate outside the dedicated CI job.");
        return;
    }
    run_required_tsan_gate();
}

#[cfg(not(unix))]
fn run_required_tsan_gate() {
    panic!("SKADI_REQUIRE_TSAN is supported only on Unix CI");
}

#[cfg(unix)]
fn run_required_tsan_gate() {
    let source = r#"
fn produce(Channel(Int) values, Int count) {
    new Int index = 0
    while index < count {
        values.send(index)
        index++
    }
}

fn consume(Channel(Int) values, Int count) Int {
    new Int total = 0
    new Int index = 0
    while index < count {
        new Int value = values.receive()
        total = total + value
        index++
    }
    return total
}

Channel(Int) values = channel(8)
Task producer_task = run produce(values, 10000)
Task(Int) consumer_task = run consume(values, 10000)
wait producer_task
new Int total = wait consumer_task
output(total)
"#;
    let tokens = lex(source).expect("lex TSan source");
    let program = parse_program(&tokens).expect("parse TSan source");
    semantic_analyze(&program).expect("semantic TSan source");
    ensure_codegen_supported(&program).expect("Task/Channel should reach codegen");
    let generated = transpile_program_to_c(&program);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let mut c_path = std::env::temp_dir();
    c_path.push(format!("skadi_channel_tsan_{stamp}.c"));
    let mut exe_path = std::env::temp_dir();
    exe_path.push(format!("skadi_channel_tsan_{stamp}"));
    fs::write(&c_path, generated).expect("write generated TSan C");

    let compiler = std::env::var("CC").unwrap_or_else(|_| "gcc".to_string());
    let compiled = Command::new(&compiler)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .args([
            "-O1",
            "-g",
            "-fno-omit-frame-pointer",
            "-fsanitize=thread",
            "-fPIE",
            "-pie",
            "-pthread",
            "-lm",
        ])
        .output()
        .expect("run TSan C compiler");
    assert!(
        compiled.status.success(),
        "required TSan compilation failed with {compiler}: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let run = Command::new("setarch")
        .args(["x86_64", "-R"])
        .arg(&exe_path)
        .env("TSAN_OPTIONS", "halt_on_error=1:exitcode=66")
        .output()
        .expect("run TSan binary");
    let _ = fs::remove_file(&c_path);
    let _ = fs::remove_file(&exe_path);

    assert!(
        run.status.success(),
        "ThreadSanitizer rejected Channel runtime: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "49995000");
}
