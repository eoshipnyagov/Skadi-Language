use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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
    assert!(help_out.contains("Full-screen interactive workflow for Skadi v1.1."));

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
        let build_dir = project_dir.join("build");
        assert!(build_dir.exists());

        let run = run_cli(&project_dir, &["run"]);
        assert!(run.status.success(), "run failed: {}", stderr_text(&run));
        let run_out = stdout_text(&run);
        assert!(run_out.contains("build ok [host]:"));
        assert!(run_out.contains("Hello from Skadi v1.1"));
    }

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
        "fn add(Int a, b) Int {\n    new sum = a + b\n    return sum\n}\n"
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
        "fn add(Int a, b) Int {\n    new sum = a + b\n    return sum\n}\n",
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
