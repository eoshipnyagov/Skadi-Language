use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn find_c_compiler() -> Option<&'static str> {
    let candidates: &[&str] = if cfg!(windows) {
        &["gcc", "clang", "cc"]
    } else {
        &["clang", "gcc", "cc"]
    };
    for c in candidates {
        if Command::new(c).arg("--version").output().is_ok() {
            return Some(c);
        }
    }
    None
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

    let run = Command::new(&exe_path)
        .output()
        .expect("run compiled binary");
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
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

    let run = Command::new(&exe_path)
        .output()
        .expect("run compiled binary");
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

    let run = Command::new(&exe_path)
        .output()
        .expect("run compiled binary");
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

    let run = Command::new(&exe_path)
        .output()
        .expect("run compiled binary");
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

    let run = Command::new(&exe_path)
        .output()
        .expect("run compiled binary");
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

    let run = Command::new(&exe_path)
        .output()
        .expect("run compiled binary");
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

    let run = Command::new(&exe_path)
        .output()
        .expect("run compiled binary");
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

    let sanitizer_flags = [
        "-fsanitize=address,undefined",
        "-fno-omit-frame-pointer",
        "-O1",
    ];
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
fn e2e_feature_mix_struct_when_iterate_incdec_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
struct Meter {
    Int value
    fn add(Int d) Int {
        my.value = my.value + d
        return my.value
    }
}
new Meter m = {value = 0}
new Int total = 0
new Int i = 0
while i < 3 {
    total = total + i
    i++
}
new Int mode = 2
when mode {
    is 1 {
        total = m.add(1)
    }
    is 2, 3 {
        total = m.add(total)
    }
    else {
        total = 0
    }
}
output(total)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("if ((__when_tmp_"));
    assert!(c.contains("i += 1;"));
    assert!(c.contains("Meter_add(&m, total)"));
    compile_c_and_run(compiler, &c, "Skadi_e2e_mix_struct_when_iterate", &[]);
}

#[test]
fn e2e_feature_mix_io_fs_branching_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Text root = "."
new Text List entries = fs.list(root)
new Int count = len(entries)
new Int idx = 0
while idx < count {
    new Text path = fs.join(root, entries[idx])
    if fs.is_dir(path) {
        output(path)
    } else {
        new Text content = read(path)
        output(content)
    }
    idx++
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_fs_list("));
    assert!(c.contains("sk_fs_join("));
    assert!(c.contains("sk_fs_is_dir("));
    assert!(c.contains("sk_read_file("));
    assert!(c.contains("sk_output_text("));
    compile_c_and_run(compiler, &c, "Skadi_e2e_mix_io_fs_branching", &[]);
}

#[test]
fn e2e_codegen_negative_output_read_inline_currently_compile_fails() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping negative e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Text root = "."
new Text List entries = fs.list(root)
new Int idx = 0
if idx < len(entries) {
    new Text p = fs.join(root, entries[idx])
    output(read(p))
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    compile_c_and_run(compiler, &c, "Skadi_e2e_output_read_inline", &["-lm"]);
}

#[test]
fn e2e_concat_output_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping negative e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Text a = "x"
new Text b = "y"
output(concat(a, b))
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    compile_c_and_run(compiler, &c, "Skadi_e2e_concat_output", &["-lm"]);
}

#[test]
fn e2e_math_core_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Float heading_deg = 45.0
new Float heading_rad = deg_to_rad(heading_deg)
new Float dx = cos(heading_rad)
new Float dy = sin(heading_rad)
new Float distance = sqrt((dx * dx) + (dy * dy))
new Float snapped = round(distance)
new Float restored_deg = rad_to_deg(atan2(dy, dx))
new Float bounded = clamp(restored_deg, 0.0, 90.0)
output(heading_rad)
output(snapped)
output(bounded)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    compile_c_and_run(compiler, &c, "Skadi_e2e_math_core", &["-lm"]);
}
