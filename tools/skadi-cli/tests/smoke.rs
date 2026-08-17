use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn cli_bin() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_skadi-cli")
            .expect("CARGO_BIN_EXE_skadi-cli should be available for integration tests"),
    )
}

fn unique_temp_dir(stem: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let dir = std::env::temp_dir().join(format!("skadi_cli_{stem}_{stamp}"));
    fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}

fn run_cli(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(cli_bin())
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("skadi-cli process should start")
}

fn run_cli_with_input(cwd: &Path, args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new(cli_bin())
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("skadi-cli process should start");
    child
        .stdin
        .take()
        .expect("debug stdin should be piped")
        .write_all(input.as_bytes())
        .expect("debug commands should be written");
    child
        .wait_with_output()
        .expect("skadi-cli process should finish")
}

fn stdout_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr_text(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn host_compiler_ready() -> bool {
    let probes: [(&str, &[&str]); 4] = [
        ("gcc", &["--version"]),
        ("clang", &["--version"]),
        ("cc", &["--version"]),
        ("cl", &[]),
    ];
    probes.iter().any(|(program, args)| {
        Command::new(program)
            .args(*args)
            .output()
            .map(|out| out.status.success() || *program == "cl")
            .unwrap_or(false)
    })
}

#[test]
fn doctor_and_target_list_smoke() {
    let temp = unique_temp_dir("doctor");

    let doctor = run_cli(&temp, &["doctor"]);
    assert!(
        doctor.status.success(),
        "doctor failed: {}",
        stderr_text(&doctor)
    );
    let doctor_out = stdout_text(&doctor);
    assert!(doctor_out.contains("skadi doctor"));
    assert!(doctor_out.contains("Host compiler candidates:"));
    assert!(doctor_out.contains("Target toolchain availability:"));

    let targets = run_cli(&temp, &["target", "list"]);
    assert!(
        targets.status.success(),
        "target list failed: {}",
        stderr_text(&targets)
    );
    let targets_out = stdout_text(&targets);
    assert!(targets_out.contains("host"));
    assert!(targets_out.contains("x86_64-w64-mingw32"));
    assert!(targets_out.contains("x86_64-unknown-linux-gnu"));

    let debug_help = run_cli(&temp, &["debug", "--help"]);
    assert!(
        debug_help.status.success(),
        "debug help failed: {}",
        stderr_text(&debug_help)
    );
    let debug_help_out = stdout_text(&debug_help);
    assert!(debug_help_out.contains("skadi-cli debug"));
    assert!(debug_help_out.contains("continue (c), step (s), quit (q)"));

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn tui_help_and_smoke_mode_work() {
    let temp = unique_temp_dir("tui_smoke");

    let help = run_cli(&temp, &["tui", "--help"]);
    assert!(
        help.status.success(),
        "tui help failed: {}",
        stderr_text(&help)
    );
    let help_out = stdout_text(&help);
    assert!(help_out.contains("skadi tui"));
    assert!(help_out.contains(concat!(
        "Full-screen interactive workflow for Skadi v",
        env!("CARGO_PKG_VERSION")
    )));

    let smoke = run_cli(&temp, &["tui", "--smoke-test"]);
    assert!(
        smoke.status.success(),
        "tui smoke failed: {}",
        stderr_text(&smoke)
    );
    assert!(stdout_text(&smoke).contains("tui smoke ok"));

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn new_check_and_optional_build_run_smoke() {
    let temp = unique_temp_dir("new_flow");

    let created = run_cli(&temp, &["new", "hello_smoke"]);
    assert!(
        created.status.success(),
        "new failed: {}",
        stderr_text(&created)
    );
    assert!(stdout_text(&created).contains("Created Skadi project"));

    let project_dir = temp.join("hello_smoke");
    assert!(project_dir.join("Skadi.toml").exists());
    assert!(project_dir.join("src").join("main.skd").exists());

    let check = run_cli(&project_dir, &["check"]);
    assert!(
        check.status.success(),
        "check failed: {}",
        stderr_text(&check)
    );
    assert!(stdout_text(&check).contains("check ok:"));

    if host_compiler_ready() {
        let build = run_cli(&project_dir, &["build"]);
        assert!(
            build.status.success(),
            "build failed: {}",
            stderr_text(&build)
        );
        assert!(stdout_text(&build).contains("build ok [host]:"));
        assert!(stdout_text(&build).contains("debug map:"));
        let build_dir = project_dir.join("build");
        assert!(build_dir.exists());
        let debug_map_path = build_dir.join("hello_smoke.skadi-debug.json");
        assert!(debug_map_path.exists());
        let debug_map: serde_json::Value = serde_json::from_slice(
            &fs::read(&debug_map_path).expect("debug map should be readable"),
        )
        .expect("debug map should be valid JSON");
        assert_eq!(debug_map["schema"], "skadi.debug-map.v1");
        assert!(
            debug_map["entries"]
                .as_array()
                .is_some_and(|entries| !entries.is_empty())
        );
        assert!(debug_map["entries"].as_array().is_some_and(|entries| {
            entries
                .iter()
                .all(|entry| entry["statement_id"].is_string())
        }));

        let run = run_cli(&project_dir, &["run"]);
        assert!(run.status.success(), "run failed: {}", stderr_text(&run));
        let run_out = stdout_text(&run);
        assert!(run_out.contains("build ok [host]:"));
        assert!(run_out.contains("Hello from Skadi"));
    }

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn debug_breakpoint_and_step_smoke() {
    if !host_compiler_ready() {
        return;
    }
    let temp = unique_temp_dir("debug_flow");
    let created = run_cli(&temp, &["new", "debug_smoke"]);
    assert!(
        created.status.success(),
        "new failed: {}",
        stderr_text(&created)
    );
    let project_dir = temp.join("debug_smoke");
    let debug = run_cli_with_input(
        &project_dir,
        &["debug", "--break", "src/main.skd:1"],
        "step\ncontinue\n",
    );
    assert!(
        debug.status.success(),
        "debug failed: {}",
        stderr_text(&debug)
    );
    let stdout = stdout_text(&debug);
    let stderr = stderr_text(&debug);
    assert!(stdout.contains("debug build ok [host]:"));
    assert!(stdout.contains("breakpoint src/main.skd:1:1"));
    assert!(stdout.contains("Hello from Skadi"));
    assert!(stderr.matches("[SKADI-DEBUG] stopped at").count() >= 2);
    assert!(stderr.contains("main.skd:1:1"));
    assert!(stderr.contains("main.skd:3:1"));

    fs::write(
        project_dir.join("src").join("worker.skd"),
        "fn helper(Int value) Int {\n    new Int result = value + 1\n    return result\n}\n",
    )
    .expect("worker source should be written");
    fs::write(
        project_dir.join("src").join("main.skd"),
        "import \"./worker.skd\"\nnew Int result = helper(6)\noutput(result)\n",
    )
    .expect("entry source should be written");
    let imported_debug = run_cli_with_input(
        &project_dir,
        &["debug", "--break", "src/worker.skd:2"],
        "continue\n",
    );
    assert!(
        imported_debug.status.success(),
        "imported debug failed: {}",
        stderr_text(&imported_debug)
    );
    assert!(
        stdout_text(&imported_debug).contains("breakpoint src/worker.skd:2:5"),
        "imported breakpoint should resolve to its source file"
    );
    let imported_stderr = stderr_text(&imported_debug);
    assert!(imported_stderr.contains("worker.skd:2:5"));
    assert!(imported_stderr.contains("#0 helper"));
    assert!(imported_stderr.contains("value: Int = 6"));

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn debug_serializes_breakpoints_from_five_tasks() {
    if !host_compiler_ready() {
        return;
    }
    let temp = unique_temp_dir("debug_tasks");
    let created = run_cli(&temp, &["new", "debug_tasks"]);
    assert!(
        created.status.success(),
        "new failed: {}",
        stderr_text(&created)
    );
    let project_dir = temp.join("debug_tasks");
    fs::write(
        project_dir.join("src").join("main.skd"),
        include_str!("../../../examples/concurrency/01_five_workers.skd"),
    )
    .expect("task debug source should be written");

    let debug = run_cli_with_input(
        &project_dir,
        &["debug", "--break", "src/main.skd:2"],
        "continue\ncontinue\ncontinue\ncontinue\ncontinue\n",
    );
    assert!(
        debug.status.success(),
        "task debug failed: {}",
        stderr_text(&debug)
    );
    let stdout = stdout_text(&debug);
    let stderr = stderr_text(&debug);
    assert!(stdout.lines().any(|line| line.trim() == "55"));
    assert_eq!(stderr.matches("[SKADI-DEBUG] stopped at").count(), 5);
    assert_eq!(stderr.matches("#0 square_and_send").count(), 5);
    for value in 1..=5 {
        assert!(
            stderr.contains(&format!("value: Int = {value}")),
            "missing worker value {value}: {stderr}"
        );
    }

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn analyze_json_reports_lifecycle_facts_and_frontend_failures() {
    let temp = unique_temp_dir("analyze_json");
    let init = run_cli(&temp, &["init"]);
    assert!(init.status.success(), "init failed: {}", stderr_text(&init));
    fs::write(
        temp.join("src").join("main.skd"),
        r#"fn worker(Channel(Int) jobs) {
    new Int value = 0
    value = jobs.receive() on error {
        if stopping {
            pass
        }
        pass
    }
    output(value)
}

Channel(Int) jobs = channel(1)
Task worker_task = run worker(jobs)
jobs.send(7)
jobs.close() on error {
    pass
}
wait worker_task
"#,
    )
    .expect("analysis source should be writable");

    let analyzed = run_cli(&temp, &["analyze", "--json"]);
    assert!(
        analyzed.status.success(),
        "analyze failed: stdout={} stderr={}",
        stdout_text(&analyzed),
        stderr_text(&analyzed),
    );
    let report: serde_json::Value =
        serde_json::from_slice(&analyzed.stdout).expect("analysis output should be JSON");
    assert_eq!(report["schema"], "skadi.analysis.v1");
    assert_eq!(report["ok"], true);
    let facts = report["facts"].as_array().expect("facts array");
    assert!(facts.iter().any(|fact| fact["code"] == "SC-AN-311"));
    assert!(
        facts
            .iter()
            .any(|fact| fact["subject"]["kind"] == "channel")
    );
    assert!(facts.iter().all(|fact| fact["id"].is_string()));

    fs::write(temp.join("src").join("main.skd"), "output(missing_value)\n")
        .expect("invalid source should be writable");
    let failed = run_cli(&temp, &["analyze", "--json"]);
    assert!(!failed.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&failed.stdout).expect("failure output should be JSON");
    assert_eq!(report["ok"], false);
    assert_eq!(report["source"], "frontend");
    assert!(
        report["diagnostics"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn quick_run_executes_one_file_without_manifest_and_forwards_arguments() {
    let temp = unique_temp_dir("quick_run");
    let help = run_cli(&temp, &["quick-run", "--help"]);
    assert!(help.status.success(), "quick-run help failed");
    assert!(stdout_text(&help).contains("without Skadi.toml"));

    if !host_compiler_ready() {
        eprintln!("Skipping quick-run native smoke: no host C compiler.");
        let _ = fs::remove_dir_all(temp);
        return;
    }

    fs::write(
        temp.join("hello.skd"),
        r#"new Text List cli_args = args()
output("single file")
output(len(cli_args))
iterate cli_args as cli_arg {
    output(cli_arg)
}
"#,
    )
    .expect("single-file source should be writable");

    let run = run_cli(
        &temp,
        &["quick-run", "hello.skd", "--", "first", "--second"],
    );
    assert!(
        run.status.success(),
        "quick-run failed: {}",
        stderr_text(&run)
    );
    let stdout = stdout_text(&run);
    assert!(stdout.contains("single file"), "{stdout}");
    assert!(stdout.contains("2"), "{stdout}");
    assert!(stdout.contains("first"), "{stdout}");
    assert!(stdout.contains("--second"), "{stdout}");
    assert!(stderr_text(&run).contains("quick-run [host]"));
    assert!(!temp.join("Skadi.toml").exists());
    assert!(!temp.join("build").exists());

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn task_runtime_builds_and_runs_through_official_cli() {
    if !host_compiler_ready() {
        eprintln!("Skipping CLI Task runtime smoke: no host C compiler.");
        return;
    }
    let temp = unique_temp_dir("task_runtime");
    let init = run_cli(&temp, &["init"]);
    assert!(init.status.success(), "init failed: {}", stderr_text(&init));
    fs::write(
        temp.join("src").join("main.skd"),
        r#"fn worker(Int worker_id, Channel(Int) events) {
    while not stopping {
        pass
    }
    events.send(worker_id)
}

fn calculate(Int base) Int {
    return base + 7
}

Channel(Int) events = channel(1)
Task worker_task = run worker(42, events)
Task(Int) result_task = run calculate(42)
stop worker_task
new Int worker_event = events.receive()
wait worker_task
new Int result = wait result_task
output(worker_event)
output(result)
output("task joined")
"#,
    )
    .expect("task entry should be writable");

    let check = run_cli(&temp, &["check"]);
    assert!(
        check.status.success(),
        "task check failed: {}",
        stderr_text(&check)
    );
    let build = run_cli(&temp, &["build"]);
    assert!(
        build.status.success(),
        "task build failed: {}",
        stderr_text(&build)
    );
    let run = run_cli(&temp, &["run"]);
    assert!(
        run.status.success(),
        "task run failed: {}",
        stderr_text(&run)
    );
    let output = stdout_text(&run);
    assert!(output.contains("42"), "{output}");
    assert!(output.contains("49"), "{output}");
    assert!(output.contains("task joined"), "{output}");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn init_and_check_smoke() {
    let temp = unique_temp_dir("init_flow");

    let init = run_cli(&temp, &["init"]);
    assert!(init.status.success(), "init failed: {}", stderr_text(&init));
    assert!(stdout_text(&init).contains("Initialized Skadi project"));
    assert!(temp.join("Skadi.toml").exists());
    assert!(temp.join("src").join("main.skd").exists());

    let check = run_cli(&temp, &["check"]);
    assert!(
        check.status.success(),
        "check failed after init: {}",
        stderr_text(&check)
    );
    assert!(stdout_text(&check).contains("check ok:"));

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn format_rewrites_project_entry() {
    let temp = unique_temp_dir("format_flow");

    let init = run_cli(&temp, &["init"]);
    assert!(init.status.success(), "init failed: {}", stderr_text(&init));

    let entry = temp.join("src").join("main.skd");
    fs::write(
        &entry,
        "fn  add( Int a,b) Int{\nnew sum= a+b\nreturn sum\n}\n",
    )
    .expect("entry should be writable");

    let format = run_cli(&temp, &["format"]);
    assert!(
        format.status.success(),
        "format failed: {}",
        stderr_text(&format)
    );
    let format_out = stdout_text(&format);
    assert!(format_out.contains("formatted"));
    assert!(format_out.contains("format ok: 1 file(s), 1 changed"));

    let rewritten = fs::read_to_string(&entry).expect("formatted file should be readable");
    assert_eq!(
        rewritten,
        "fn add(Int a, b) returns Int {\n    new sum = a + b\n    return sum\n}\n"
    );

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn format_check_passes_for_canonical_file() {
    let temp = unique_temp_dir("format_check_ok");

    let init = run_cli(&temp, &["init"]);
    assert!(init.status.success(), "init failed: {}", stderr_text(&init));

    let entry = temp.join("src").join("main.skd");
    fs::write(
        &entry,
        "fn add(Int a, b) returns Int {\n    new sum = a + b\n    return sum\n}\n",
    )
    .expect("entry should be writable");

    let check = run_cli(&temp, &["format", "--check"]);
    assert!(
        check.status.success(),
        "format --check failed unexpectedly: {}",
        stderr_text(&check)
    );
    let out = stdout_text(&check);
    assert!(out.contains("ok "));
    assert!(out.contains("format check ok: 1 file(s)"));

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn format_check_fails_without_rewriting_file() {
    let temp = unique_temp_dir("format_check_fail");

    let init = run_cli(&temp, &["init"]);
    assert!(init.status.success(), "init failed: {}", stderr_text(&init));

    let entry = temp.join("src").join("main.skd");
    let ugly = "fn  add( Int a,b) Int{\nnew sum= a+b\nreturn sum\n}\n";
    fs::write(&entry, ugly).expect("entry should be writable");

    let check = run_cli(&temp, &["format", "--check"]);
    assert!(
        !check.status.success(),
        "format --check should fail for non-canonical file"
    );
    assert!(stdout_text(&check).contains("needs format"));
    assert!(stderr_text(&check).contains("format check failed: 1 file(s) need formatting."));

    let after = fs::read_to_string(&entry).expect("entry should remain readable");
    assert_eq!(after, ugly);

    let _ = fs::remove_dir_all(temp);
}
