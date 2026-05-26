use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn find_c_compiler() -> Option<&'static str> {
    ["clang", "gcc", "cc"]
        .into_iter()
        .find(|c| Command::new(c).arg("--version").output().is_ok())
}

fn compiler_supports_flags(compiler: &str, flags: &[&str]) -> bool {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path: PathBuf = std::env::temp_dir();
    c_path.push(format!("Skadi_flag_probe_{stamp}.c"));
    let mut exe_path: PathBuf = std::env::temp_dir();
    exe_path.push(format!("Skadi_flag_probe_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    if fs::write(&c_path, "int main(void){return 0;}\n").is_err() {
        return false;
    }

    let mut cmd = Command::new(compiler);
    cmd.arg(&c_path).arg("-o").arg(&exe_path);
    for flag in flags {
        cmd.arg(flag);
    }
    let ok = cmd.output().map(|o| o.status.success()).unwrap_or(false);

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
    ok
}

fn compile_c_and_run(compiler: &str, c_src: &str, stem: &str, extra_flags: &[&str]) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path: PathBuf = std::env::temp_dir();
    c_path.push(format!("{stem}_{stamp}.c"));
    let mut exe_path: PathBuf = std::env::temp_dir();
    exe_path.push(format!("{stem}_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    fs::write(&c_path, c_src).expect("write C source");

    let mut compile_cmd = Command::new(compiler);
    compile_cmd.arg(&c_path).arg("-o").arg(&exe_path);
    for flag in extra_flags {
        compile_cmd.arg(flag);
    }
    let compile = compile_cmd.output().expect("run C compiler");
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&exe_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
}

fn compile_skadi_and_run(compiler: &str, src: &str, stem: &str, extra_flags: &[&str]) {
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    compile_c_and_run(compiler, &c, stem, extra_flags);
}

#[test]
fn e2e_skadi_to_c_binary_builds() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Int x = 2
new Int y = x + 3
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path: PathBuf = std::env::temp_dir();
    c_path.push(format!("Skadi_e2e_{stamp}.c"));
    let mut exe_path: PathBuf = std::env::temp_dir();
    exe_path.push(format!("Skadi_e2e_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    fs::write(&c_path, c).expect("write C source");

    let compile = Command::new(compiler)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("run C compiler");
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&exe_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
}

#[test]
fn e2e_skadi_text_builtins_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Text t = "weather station"
new bool has_station = contains(t, "station")
new Int idx = find(t, "ther")
new Text part = slice(t, 3, 7)
new Int n = len(part)
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path: PathBuf = std::env::temp_dir();
    c_path.push(format!("Skadi_e2e_text_{stamp}.c"));
    let mut exe_path: PathBuf = std::env::temp_dir();
    exe_path.push(format!("Skadi_e2e_text_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    fs::write(&c_path, c).expect("write C source");

    let compile = Command::new(compiler)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("run C compiler");
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&exe_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
}

#[test]
fn e2e_skadi_text_edge_bounds_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Text t = "abc"
new Text s1 = slice(t, -2, 2)
new Text s2 = slice(t, 5, 1)
new Int i1 = find(t, "")
new bool c1 = contains(t, "")
new Int n1 = len(s1)
new Int n2 = len(s2)
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path: PathBuf = std::env::temp_dir();
    c_path.push(format!("Skadi_e2e_text_edge_{stamp}.c"));
    let mut exe_path: PathBuf = std::env::temp_dir();
    exe_path.push(format!("Skadi_e2e_text_edge_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    fs::write(&c_path, c).expect("write C source");

    let compile = Command::new(compiler)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("run C compiler");
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&exe_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
}

#[test]
fn e2e_skadi_text_utf8_byte_semantics_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Text t = "Привет"
new Int n = len(t)
new char b0 = t[0]
new Text head = slice(t, 0, 4)
new Int idx = find(t, "вет")
new bool has = contains(t, "рив")
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    compile_c_and_run(compiler, &c, "Skadi_e2e_text_utf8_bytes", &[]);
}

#[test]
fn e2e_skadi_list_and_when_edge_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
label ErrorCode {
    Ok
    EmptyQueue
}

new i32 List values = []
new i32 current = 0
current = values.pop() on error {
    current = -1
}
values.push(7)
values.push(9)
new Int count = len(values)
new Int mode = 99
when mode {
    is 1 {
        current = 1
    }
    is 2, 3 {
        current = 2
    }
    else {
        current = current + count
    }
}
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path: PathBuf = std::env::temp_dir();
    c_path.push(format!("Skadi_e2e_list_when_edge_{stamp}.c"));
    let mut exe_path: PathBuf = std::env::temp_dir();
    exe_path.push(format!("Skadi_e2e_list_when_edge_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    fs::write(&c_path, c).expect("write C source");

    let compile = Command::new(compiler)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("run C compiler");
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&exe_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
}

#[test]
fn e2e_skadi_checked_index_bounds_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new i32 List xs = [5, 6]
new i32 ok = xs[1]
new i32 out = xs[42]
new Text t = "xy"
new char c_ok = t[1]
new char c_out = t[10]
new Int sum = ok + out
if c_ok == c_out {
    sum = sum + 1
} else {
    sum = sum + 2
}
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path: PathBuf = std::env::temp_dir();
    c_path.push(format!("Skadi_e2e_checked_index_{stamp}.c"));
    let mut exe_path: PathBuf = std::env::temp_dir();
    exe_path.push(format!("Skadi_e2e_checked_index_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    fs::write(&c_path, c).expect("write C source");

    let compile = Command::new(compiler)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("run C compiler");
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&exe_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
}

#[test]
fn e2e_skadi_fs_list_and_is_dir_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Text root = "."
new Text List entries = fs.list(root)
new bool has_any = false
for e in entries {
    has_any = fs.is_dir(e) or has_any
}
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path: PathBuf = std::env::temp_dir();
    c_path.push(format!("Skadi_e2e_fs_{stamp}.c"));
    let mut exe_path: PathBuf = std::env::temp_dir();
    exe_path.push(format!("Skadi_e2e_fs_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    fs::write(&c_path, c).expect("write C source");

    let compile = Command::new(compiler)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("run C compiler");
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&exe_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
}

#[test]
fn e2e_sanitized_runtime_stress_list_text_path() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping sanitizer e2e test: no clang/gcc/cc in PATH.");
        return;
    };

    let sanitizer_flags = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer", "-O1"];
    if !compiler_supports_flags(compiler, &sanitizer_flags) {
        eprintln!("Skipping sanitizer e2e test: compiler does not support ASan/UBSan flags.");
        return;
    }

    let src = r#"
new Path root = "."
new Path List entries = fs.list(root)
new Int List sizes = []
new Int total = 0

iterate entries as entry {
    new Int n = len(entry)
    sizes.push(n)
}

new Int i = 0
while i < len(sizes) {
    total = total + sizes[i]
    i = i + 1
}

new Int x = 0
while len(sizes) > 0 {
    x = sizes.pop() on error {
        x = -1
    }
}

new Text msg = concat("total=", "ok")
output(msg)
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    compile_c_and_run(compiler, &c, "Skadi_e2e_sanitized", &sanitizer_flags);
}

#[test]
fn e2e_feature_mix_inc_dec_with_loop_flow() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Int i = 0
new Int acc = 0
while i < 10 {
    i = i + 1
    if i mod 2 != 0 {
        if i < 8 {
            acc = acc + i
        }
    }
}
"#;

    compile_skadi_and_run(compiler, src, "Skadi_e2e_feature_mix_inc_dec", &[]);
}

#[test]
fn e2e_feature_mix_iterate_when_pass_and_text() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Text List items = ["alpha", "beta", "gamma"]
new Int score = 0
iterate items as item {
    when len(item) {
        is 0 {
            score = score + 0
        }
        else {
            score = score + len(item)
        }
    }
}
output(score)
"#;

    compile_skadi_and_run(compiler, src, "Skadi_e2e_feature_mix_iterate_when", &[]);
}

#[test]
fn e2e_feature_mix_danger_on_error_inside_loop() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn div_safe(Int a, Int b) Int {
    if b == 0 {
        return error ZeroDivision
    } else {
        return a div b
    }
}

new i32 List divisors = [2, 1, 0, 4]
new Int i = 0
new Int total = 0
new Int part = 0
while i < len(divisors) {
    part = div_safe(20, divisors[i]) on error {
        part = 0
    }
    total = total + part
    i = i + 1
}
output(total)
"#;

    compile_skadi_and_run(compiler, src, "Skadi_e2e_feature_mix_danger_loop", &[]);
}

#[test]
fn e2e_when_find_expression_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Text t = "alpha"
new Int out = 0
when find(t, "ph") {
    is -1 {
        out = 0
    }
    else {
        out = 1
    }
}
output(out)
"#;

    compile_skadi_and_run(compiler, src, "Skadi_e2e_when_find", &[]);
}

#[test]
fn e2e_feature_mix_fs_text_when_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Text root = "."
new Text List entries = fs.list(root)
new Int i = 0
new Int files = 0
while i < len(entries) {
    new Text p = entries[i]
    when fs.is_dir(p) {
        is true {
            files = files + 0
        }
        else {
            files = files + 1
        }
    }
    i = i + 1
}
output(files)
"#;

    compile_skadi_and_run(compiler, src, "Skadi_e2e_feature_mix_fs_text_when", &[]);
}

#[test]
fn e2e_feature_mix_danger_when_list_pop_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
label ErrorCode {
    Ok
    BadInput
}

danger fn parse_nonzero(Int x) Int {
    if x == 0 {
        return error BadInput
    } else {
        return x
    }
}

new Int List q = [3, 2, 0, 1]
new Int total = 0
new Int v = 0
new Int parsed = 0
while len(q) > 0 {
    v = q.pop() on error {
        v = 0
    }
    parsed = parse_nonzero(v) on error {
        parsed = 0
    }
    when parsed {
        is 0 {
            total = total + 0
        }
        else {
            total = total + parsed
        }
    }
}
output(total)
"#;

    compile_skadi_and_run(compiler, src, "Skadi_e2e_feature_mix_danger_when_pop", &[]);
}

#[test]
fn e2e_loop_control_and_inc_dec_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Int i = 0
new Int acc = 0
loop {
    i++
    if i mod 2 == 0 {
        pass
        continue
    }
    acc = acc + i
    if i >= 7 {
        break
    }
}
output(acc)
"#;

    compile_skadi_and_run(compiler, src, "Skadi_e2e_loop_control_incdec", &[]);
}

