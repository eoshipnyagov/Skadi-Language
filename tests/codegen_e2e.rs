use std::fs;
use std::path::{Path, PathBuf};
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
    candidates
        .iter()
        .find(|&&c| Command::new(c).arg("--version").output().is_ok())
        .copied()
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
    let run = compile_c_and_execute(compiler, c_src, stem, extra_flags, &[], None);
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}

fn compile_c_and_execute(
    compiler: &str,
    c_src: &str,
    stem: &str,
    extra_flags: &[&str],
    run_args: &[&str],
    cwd: Option<&Path>,
) -> std::process::Output {
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

    let mut run_cmd = Command::new(&exe_path);
    run_cmd.args(run_args);
    if let Some(cwd) = cwd {
        run_cmd.current_dir(cwd);
    }
    let run = run_cmd.output().expect("run compiled binary");

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
    run
}

fn compile_showcase_to_c(src: &str) -> String {
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    transpile_program_to_c(&program)
}

fn value_after_label<'a>(stdout: &'a str, label: &str) -> Option<&'a str> {
    let mut lines = stdout.lines();
    while let Some(line) = lines.next() {
        if line.trim() == label {
            return lines.next().map(str::trim);
        }
    }
    None
}

struct CliShowcaseCase<'a> {
    name: &'a str,
    src: &'a str,
    extra_flags: &'a [&'a str],
    run_args: &'a [&'a str],
    cwd: Option<&'a Path>,
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
    Ok = 0
    EmptyQueue = 1
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
new Angle heading = 45deg
new Float dx = cos(heading)
new Float dy = sin(heading)
new Float distance = sqrt((dx * dx) + (dy * dy))
new Float snapped = round(distance)
new Angle restored = atan2(dy, dx)
new Float restored_deg = rad_to_deg(restored)
new Float bounded = clamp(restored_deg, 0.0, 90.0)
output(restored_deg)
output(snapped)
output(bounded)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    compile_c_and_run(compiler, &c, "Skadi_e2e_math_core", &["-lm"]);
}

#[test]
fn e2e_complete_math_and_units_slice_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping complete math/units e2e test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Float fraction = fract(-1.25)
new Float interpolated = lerp(10.0, 20.0, 0.25)
new Float mapped = remap(50.0, 0.0, 100.0, -1.0, 1.0)
new Float eased = smoothstep(0.0, 1.0, 0.5)
new Angle wrapped = normalize_angle(540deg)
new Bool invalid = is_nan(sqrt(-1.0))

new Duration tick = 1ns
new Duration pulse = 10us
new Duration frame = 1s / 4
new Float frame_ratio = 1s / 250ms

new ByteSize archive = 1tb
new ByteSize half_archive = archive / 2
new Float size_ratio = 1mb / 256kb

output(fraction)
output(interpolated)
output(mapped)
output(eased)
output(rad_to_deg(wrapped))
output(invalid)
output(as_nanoseconds(tick + pulse + frame))
output(frame_ratio)
output(as_bytes(half_archive))
output(size_ratio)
"#;
    let c = compile_showcase_to_c(src);
    let run = compile_c_and_execute(
        compiler,
        &c,
        "Skadi_e2e_complete_math_units",
        &["-lm"],
        &[],
        None,
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let lines = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 10, "{lines:?}");
    let number = |index: usize| lines[index].parse::<f64>().expect("numeric output");
    assert!((number(0) - 0.75).abs() < 1e-9, "{lines:?}");
    assert!((number(1) - 12.5).abs() < 1e-9, "{lines:?}");
    assert!(number(2).abs() < 1e-9, "{lines:?}");
    assert!((number(3) - 0.5).abs() < 1e-9, "{lines:?}");
    assert!((number(4) + 180.0).abs() < 1e-6, "{lines:?}");
    assert_eq!(lines[5], "true");
    assert_eq!(lines[6], "250010001");
    assert!((number(7) - 4.0).abs() < 1e-9, "{lines:?}");
    assert_eq!(lines[8], "549755813888");
    assert!((number(9) - 4.0).abs() < 1e-9, "{lines:?}");
}

#[test]
fn e2e_angle_literals_and_trigonometry_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Angle e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Angle heading = 90deg
new Angle half = heading / 2
new Angle divisor = 36deg
new Float dx = cos(half)
new Float dy = sin(half)
new Angle measured = atan2(dy, dx)
output(rad_to_deg(measured))
output(heading / divisor)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    let run = compile_c_and_execute(compiler, &c, "Skadi_e2e_angle", &["-lm"], &[], None);
    assert!(run.status.success());
    let values = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(|line| line.parse::<f64>().expect("numeric Angle output"))
        .collect::<Vec<_>>();
    assert!((values[0] - 45.0).abs() < 1e-6, "{:?}", values);
    assert!((values[1] - 2.5).abs() < 1e-6, "{:?}", values);
}

#[test]
fn e2e_vector_math_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping vector e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Vec3 x_axis = {x = 1.0, y = 0.0, z = 0.0}
new Vec3 y_axis = {x = 0.0, y = 1.0, z = 0.0}
new Vec3 normal = normalize(cross(x_axis, y_axis))
new Vec3 zero = {x = 0.0, y = 0.0, z = 0.0}
new Vec3 safe_zero = normalize(zero)
output(normal.z)
output(length(normal))
output(distance(x_axis, y_axis))
output(safe_zero.x)
"#;
    let c = compile_showcase_to_c(src);
    let run = compile_c_and_execute(compiler, &c, "Skadi_e2e_vector", &["-lm"], &[], None);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let values = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(|line| line.parse::<f64>().expect("numeric vector output"))
        .collect::<Vec<_>>();
    assert!((values[0] - 1.0).abs() < 1e-9, "{values:?}");
    assert!((values[1] - 1.0).abs() < 1e-9, "{values:?}");
    assert!((values[2] - 2.0_f64.sqrt()).abs() < 1e-5, "{values:?}");
    assert!(values[3].abs() < 1e-9, "{values:?}");
}

#[test]
fn e2e_char_literals_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Char e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Char letter = 'a'
new Char List letters = [letter, '\n']
output(letter)
output(letters[0])
"#;
    let c = compile_showcase_to_c(src);
    let run = compile_c_and_execute(compiler, &c, "Skadi_e2e_char", &[], &[], None);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let lines = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(lines, ["a", "a"]);
}

#[test]
fn e2e_power_operator_lowers_to_math_runtime() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Float squared = 3.0 ^ 2.0
output(squared)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("#include <math.h>"));
    assert!(c.contains("powf(3, 2)"));

    let run = compile_c_and_execute(
        compiler,
        &c,
        "Skadi_e2e_power_operator",
        &["-lm"],
        &[],
        None,
    );
    assert!(run.status.success());
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "9.000000");
}

#[test]
fn e2e_fixed_width_bit_operations_are_predictable() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new i8 raw_value = -1
new i8 logical = bit_shift_right(raw_value, 1)
new u8 flags = 0
flags = bit_set(flags, 3)
flags = bit_toggle(flags, 0)
output(logical)
output(flags)
output(bit_is_set(flags, 3))
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    let run = compile_c_and_execute(compiler, &c, "Skadi_e2e_bits", &[], &[], None);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["127", "9", "true"]
    );
}

#[test]
fn e2e_time_duration_runtime_is_monotonic() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = r#"
new Time started_at = now()
sleep(5ms)
new Duration first_elapsed = elapsed(started_at)
new Time checkpoint = now()
delay(1ms)
new Duration second_elapsed = now() - checkpoint
new Bool first_ok = first_elapsed >= 1ms
new Bool second_ok = second_elapsed >= 1ms
output(first_ok and second_ok)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    let run = compile_c_and_execute(compiler, &c, "Skadi_e2e_time_duration", &[], &[], None);
    assert!(
        run.status.success(),
        "time runtime failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "true");
}

#[test]
fn e2e_v1_1_toolbox_showcase_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };
    let src = include_str!("../benchmarks/bench_10_v1_1_toolbox.skd");
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_list_Waypoint_free(&route);"));
    assert!(c.contains("sk_free_text((void*)summary);"));
    compile_c_and_run(compiler, &c, "Skadi_e2e_v1_1_toolbox", &["-lm"]);
}

#[test]
fn e2e_stable_showcase_subset_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping showcase subset e2e test: no clang/gcc/cc in PATH.");
        return;
    };

    let stable_showcases: &[(&str, &str, &[&str])] = &[
        (
            "bench_06_struct_account",
            include_str!("../benchmarks/bench_06_struct_account.skd"),
            &[],
        ),
        (
            "bench_07_struct_list",
            include_str!("../benchmarks/bench_07_struct_list.skd"),
            &[],
        ),
        (
            "bench_08_path_list_helpers",
            include_str!("../benchmarks/bench_08_path_list_helpers.skd"),
            &[],
        ),
        (
            "bench_09_math_navigation",
            include_str!("../benchmarks/bench_09_math_navigation.skd"),
            &["-lm"],
        ),
    ];

    for (name, src, extra_flags) in stable_showcases {
        let c = compile_showcase_to_c(src);
        compile_c_and_run(compiler, &c, name, extra_flags);
    }
}

#[test]
fn e2e_variadic_output_and_nominal_when_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping variadic output e2e test: no C compiler in PATH.");
        return;
    };
    let src = r#"
tag Direction {
    North
    South
}

fn direction_name() returns Text {
    return "north"
}

fn is_ready() returns Bool {
    return true
}

struct Compass {
    Text heading

    fn name() returns Text {
        return my.heading
    }
}

new Direction direction = Direction.North
new Compass compass = {heading = "north"}
when direction {
    is North {
        output("direction=", direction_name(), "/", compass.name(), ", value=", 7, ", ready=", is_ready())
    }
    is South {
        output("direction=south")
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(!c.contains("_Generic"), "{c}");
    assert!(c.contains("sk_output_text_part(direction_name())"), "{c}");
    assert!(
        c.contains("sk_output_text_part(Compass_name(&compass))"),
        "{c}"
    );
    assert!(c.contains("sk_output_bool_part(is_ready())"), "{c}");
    let run = compile_c_and_execute(compiler, &c, "Skadi_e2e_variadic_output", &[], &[], None);
    assert!(
        run.status.success(),
        "variadic output runtime failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim_end(),
        "direction=north/north, value=7, ready=true"
    );
}

#[test]
fn e2e_cli_driven_showcase_subset_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping CLI-driven showcase e2e test: no clang/gcc/cc in PATH.");
        return;
    };

    let manifest_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let showcase_data_root = manifest_root.join("benchmarks").join("showcase-data");
    let tree_fixture = showcase_data_root.join("tree_fixture");
    let weather_fixture = showcase_data_root.join("sample_weather.txt");
    let weather_text =
        fs::read_to_string(&weather_fixture).expect("weather fixture should be readable");
    let expected_weather_chars = weather_text.chars().count().to_string();
    let expected_weather_lines = weather_text.lines().count().to_string();

    let showcase_cases = [
        CliShowcaseCase {
            name: "bench_01_tree",
            src: include_str!("../benchmarks/bench_01_tree.skd"),
            extra_flags: &[],
            run_args: &["--dirs-only", "--depth-1"],
            cwd: Some(tree_fixture.as_path()),
        },
        CliShowcaseCase {
            name: "bench_02_read_stats",
            src: include_str!("../benchmarks/bench_02_read_stats.skd"),
            extra_flags: &[],
            run_args: &["--input", "benchmarks/showcase-data/sample_weather.txt"],
            cwd: Some(manifest_root),
        },
        CliShowcaseCase {
            name: "bench_03_find_count",
            src: include_str!("../benchmarks/bench_03_find_count.skd"),
            extra_flags: &[],
            run_args: &[
                "--input",
                "benchmarks/showcase-data/sample_weather.txt",
                "--needle",
                "temperature",
            ],
            cwd: Some(manifest_root),
        },
        CliShowcaseCase {
            name: "bench_04_sum_ints",
            src: include_str!("../benchmarks/bench_04_sum_ints.skd"),
            extra_flags: &[],
            run_args: &["--small"],
            cwd: Some(manifest_root),
        },
        CliShowcaseCase {
            name: "bench_05_push_pop",
            src: include_str!("../benchmarks/bench_05_push_pop.skd"),
            extra_flags: &[],
            run_args: &["--small"],
            cwd: Some(manifest_root),
        },
    ];

    for case in showcase_cases {
        let c = compile_showcase_to_c(case.src);
        let run = compile_c_and_execute(
            compiler,
            &c,
            case.name,
            case.extra_flags,
            case.run_args,
            case.cwd,
        );
        assert!(
            run.status.success(),
            "Binary execution failed for {}: {}",
            case.name,
            String::from_utf8_lossy(&run.stderr)
        );
        let stdout = String::from_utf8_lossy(&run.stdout);
        match case.name {
            "bench_01_tree" => {
                assert!(stdout.contains("[D] ."));
                assert!(stdout.contains("[D] ./alpha"));
                assert!(!stdout.contains("[F] "));
            }
            "bench_02_read_stats" => {
                assert!(stdout.contains("file: benchmarks/showcase-data/sample_weather.txt"));
                assert_eq!(
                    value_after_label(&stdout, "chars:"),
                    Some(expected_weather_chars.as_str())
                );
                assert_eq!(
                    value_after_label(&stdout, "lines:"),
                    Some(expected_weather_lines.as_str())
                );
            }
            "bench_03_find_count" => {
                assert!(stdout.contains("needle: temperature"));
                assert!(stdout.contains("count:"));
                assert!(stdout.contains("3"));
            }
            "bench_04_sum_ints" => {
                assert!(stdout.contains("n:"));
                assert!(stdout.contains("50000"));
                assert!(stdout.contains("sum:"));
                assert!(stdout.contains("1249975000"));
            }
            "bench_05_push_pop" => {
                assert!(stdout.contains("n:"));
                assert!(stdout.contains("50000"));
                assert!(stdout.contains("popped:"));
                assert!(stdout.contains("50000"));
            }
            _ => unreachable!("unexpected showcase case"),
        }
    }
    assert!(weather_fixture.exists());
}
