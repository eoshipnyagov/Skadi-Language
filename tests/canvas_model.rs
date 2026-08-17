use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use v01::codegen::{ensure_codegen_supported, transpile_program_to_c};
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

static NEXT_ARTIFACT: AtomicU64 = AtomicU64::new(0);

fn parse_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex Canvas source");
    parse_program(&tokens).expect("parse Canvas source")
}

fn semantic_ok(source: &str) -> v01::ast_nodes::Program {
    let program = parse_ok(source);
    semantic_analyze(&program).expect("semantic Canvas source");
    program
}

fn semantic_err(source: &str) -> String {
    let program = parse_ok(source);
    semantic_analyze(&program).expect_err("expected Canvas semantic error")
}

fn find_c_compiler() -> Option<&'static str> {
    ["gcc", "clang", "cc"]
        .iter()
        .find(|&&compiler| Command::new(compiler).arg("--version").output().is_ok())
        .copied()
}

fn compile_and_run(compiler: &str, c_source: &str) -> std::process::Output {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let sequence = NEXT_ARTIFACT.fetch_add(1, Ordering::Relaxed);
    let artifact = format!("{}_{}_{}", std::process::id(), stamp, sequence);
    let c_path = std::env::temp_dir().join(format!("skadi_canvas_{artifact}.c"));
    let mut exe_path = std::env::temp_dir().join(format!("skadi_canvas_{artifact}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }
    fs::write(&c_path, c_source).expect("write Canvas C");
    let compile = Command::new(compiler)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .arg("-lm")
        .output()
        .expect("compile Canvas C");
    assert!(
        compile.status.success(),
        "Canvas C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&exe_path).output().expect("run Canvas binary");
    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
    run
}

const HEADLESS_SCENE: &str = r##"
Canvas frame = canvas(16, 16)
new Color background = color_hex("#1d1f21", 255)
new Color accent = Color.terminal_bright_yellow
new Rect panel = rect(1.0, 1.0, 14.0, 14.0)
new Vec2 center = {x = 8.0, y = 8.0}

frame.clear(background)
frame.fill_rect(panel, Color.terminal_blue)
frame.rect(rect(-2.0, -2.0, 8.0, 8.0), Color.terminal_green)
frame.circle(center, 5.0, accent)
frame.fill_circle(center, 2.0, color(213, 78, 83, 192))
output(frame.checksum())
"##;

const WINDOW_SCENE: &str = r#"
Canvas frame = canvas(64, 48)
Window window = windows.open("Skadi Canvas", 64, 48)
frame.clear(Color.terminal_black)
frame.line({x = 4.0, y = 4.0}, {x = 59.0, y = 43.0}, Color.terminal_bright_cyan)
window.present(direct frame)
window.close()
"#;

#[test]
fn canvas_frontend_accepts_palette_hex_and_shape_primitives() {
    let program = semantic_ok(HEADLESS_SCENE);
    let warnings = semantic_style_warnings(&program);
    assert!(
        warnings
            .iter()
            .all(|warning| !warning.contains("non-canonical type spelling")),
        "unexpected Canvas style warnings: {warnings:?}"
    );
    ensure_codegen_supported(&program).expect("Canvas should reach codegen");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_color_hex(\"#1d1f21\", 255)"));
    assert!(c.contains("Color_terminal_bright_yellow"));
    assert!(c.contains("sk_canvas_fill_rect(&frame"));
    assert!(c.contains("sk_canvas_circle(&frame"));
    assert!(c.contains("sk_canvas_fill_circle(&frame"));
    assert!(c.contains("sk_canvas_destroy(&frame)"));
    assert!(!c.contains("StretchDIBits"));
    assert!(!c.contains("typedef struct {\n    bool open;"));
}

#[test]
fn window_backend_uses_explicit_canvas_borrow_and_win32_presenter() {
    let program = semantic_ok(WINDOW_SCENE);
    ensure_codegen_supported(&program).expect("Window scene should reach codegen");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_window_open(\"Skadi Canvas\", 64, 48)"));
    assert!(c.contains("sk_window_present(&window, &frame)"));
    assert!(c.contains("StretchDIBits"));
    assert!(c.contains("sk_window_destroy(&window)"));
}

#[test]
fn canvas_resources_require_explicit_borrows_and_cannot_be_copied() {
    let value_parameter = semantic_err(
        r#"
fn draw(Canvas frame) {
    pass
}
"#,
    );
    assert!(
        value_parameter.contains("must use 'direct'"),
        "{value_parameter}"
    );

    let copied = semantic_err(
        r#"
Canvas first = canvas(8, 8)
Canvas second = canvas(8, 8)
second = first
"#,
    );
    assert!(
        copied.contains("cannot be reassigned or copied"),
        "{copied}"
    );

    let present_without_borrow = semantic_err(
        r#"
Canvas frame = canvas(8, 8)
Window window = windows.open("Canvas", 8, 8)
window.present(frame)
"#,
    );
    assert!(
        present_without_borrow.contains("requires explicit 'direct <Canvas>'"),
        "{present_without_borrow}"
    );

    let view_mutation = semantic_err(
        r#"
fn erase(view Canvas frame) {
    frame.clear(Color.black)
}
"#,
    );
    assert!(
        view_mutation.contains("view parameter 'frame' cannot call mutating method 'clear'"),
        "{view_mutation}"
    );
}

#[test]
fn canvas_borrowed_functions_separate_mutation_from_observation() {
    let program = semantic_ok(
        r#"
fn paint(direct Canvas frame) {
    frame.clear(Color.terminal_black)
}

fn fingerprint(view Canvas frame) returns Int {
    return frame.checksum()
}

Canvas frame = canvas(8, 8)
paint(direct frame)
new Int checksum = fingerprint(view frame)
output(checksum)
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkInt paint(SkCanvas * frame)"));
    assert!(c.contains("SkInt fingerprint(const SkCanvas * frame)"));
    assert!(c.contains("sk_canvas_clear(frame, Color_terminal_black)"));
    assert!(c.contains("sk_canvas_checksum(frame)"));
}

#[test]
fn move_transfers_canvas_ownership_and_rejects_the_old_name() {
    let moved = semantic_err(
        r#"
fn finish(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(8, 8)
finish(move frame)
output(frame.checksum())
"#,
    );
    assert!(moved.contains("resource 'frame' was moved"), "{moved}");
    assert!(moved.contains("no longer available"), "{moved}");

    let partial = semantic_err(
        r#"
fn finish(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(8, 8)
new Bool hand_off = true
if hand_off {
    finish(move frame)
}
output(frame.checksum())
"#,
    );
    assert!(
        partial.contains("available only on some control-flow paths"),
        "{partial}"
    );
}

#[test]
fn canvas_factory_returns_ownership_end_to_end() {
    let program = semantic_ok(
        r#"
fn make_frame(Int width, Int height) returns Canvas {
    Canvas frame = canvas(width, height)
    frame.clear(Color.terminal_blue)
    return move frame
}

Canvas frame = make_frame(8, 8)
output(frame.checksum())
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkCanvas make_frame("), "{c}");
    assert!(c.contains("sk_canvas_move(&frame)"), "{c}");
    assert!(c.contains("sk_canvas_destroy(&frame)"), "{c}");

    let Some(compiler) = find_c_compiler() else {
        return;
    };
    let run = compile_and_run(compiler, &c);
    assert!(
        run.status.success(),
        "Canvas move runtime failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!String::from_utf8_lossy(&run.stdout).trim().is_empty());
}

#[test]
fn ownership_showcase_builds_and_runs() {
    let path = format!(
        "{}/examples/ownership/01_move_canvas_factory.skd",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = fs::read_to_string(path).expect("read ownership showcase");
    let program = semantic_ok(&source);
    let c = transpile_program_to_c(&program);

    let Some(compiler) = find_c_compiler() else {
        return;
    };
    let run = compile_and_run(compiler, &c);
    assert!(
        run.status.success(),
        "ownership showcase failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!String::from_utf8_lossy(&run.stdout).trim().is_empty());
}

#[test]
fn move_from_repeating_loop_is_rejected() {
    let error = semantic_err(
        r#"
fn finish(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(8, 8)
loop {
    finish(move frame)
    break
}
"#,
    );
    assert!(
        error.contains("cannot be moved from a repeating loop body"),
        "{error}"
    );
}

#[test]
fn move_requires_explicit_owning_boundaries() {
    let missing_call_marker = semantic_err(
        r#"
fn finish(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(8, 8)
finish(frame)
"#,
    );
    assert!(
        missing_call_marker.contains("requires explicit 'move <identifier>'"),
        "{missing_call_marker}"
    );

    let missing_return_marker = semantic_err(
        r#"
fn make_frame() returns Canvas {
    Canvas frame = canvas(8, 8)
    return frame
}
"#,
    );
    assert!(
        missing_return_marker.contains("requires explicit 'return move <identifier>'"),
        "{missing_return_marker}"
    );

    let value_move = semantic_err(
        r#"
new Int answer = 42
new Int copy = move answer
"#,
    );
    assert!(
        value_move.contains("ordinary values are copied"),
        "{value_move}"
    );
}

#[test]
fn window_lifecycle_requires_on_error_after_conditional_close() {
    let unhandled = semantic_err(
        r#"
Canvas frame = canvas(8, 8)
Window window = windows.open("Canvas", 8, 8)
new Bool should_close = true

if should_close {
    window.close()
}

window.present(direct frame)
"#,
    );
    assert!(
        unhandled.contains("resource 'window' may be closed"),
        "{unhandled}"
    );
    assert!(unhandled.contains("with 'on error'"), "{unhandled}");

    let handled = semantic_ok(
        r#"
Canvas frame = canvas(8, 8)
Window window = windows.open("Canvas", 8, 8)
new Bool should_close = true

if should_close {
    window.close()
}

window.present(direct frame) on error {
    pass
}
"#,
    );
    let c = transpile_program_to_c(&handled);
    assert!(
        c.contains("if (sk_window_present(&window, &frame) != 0)"),
        "{c}"
    );
}

#[test]
fn definitely_closed_window_operation_is_handled_and_warned() {
    let source = r#"
Canvas frame = canvas(8, 8)
Window window = windows.open("Canvas", 8, 8)
window.close()

window.present(direct frame) on error {
    pass
}
"#;
    let program = semantic_ok(source);
    let warnings = semantic_style_warnings(&program);
    assert!(
        warnings.iter().any(|warning| {
            warning.contains("resource 'window' is already closed")
                && warning.contains("guaranteed to enter its 'on error' handler")
        }),
        "{warnings:?}"
    );
}

#[test]
fn early_return_keeps_the_fallthrough_window_open() {
    semantic_ok(
        r#"
fn render(Bool should_close) returns Int {
    Canvas frame = canvas(8, 8)
    Window window = windows.open("Canvas", 8, 8)

    if should_close {
        window.close()
        return 0
    }

    window.present(direct frame)
    return 0
}
"#,
    );
}

#[test]
fn canvas_headless_scene_builds_runs_and_has_stable_checksum() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping Canvas runtime e2e: no C compiler in PATH.");
        return;
    };
    let program = semantic_ok(HEADLESS_SCENE);
    let c = transpile_program_to_c(&program);
    let run = compile_and_run(compiler, &c);
    assert!(
        run.status.success(),
        "Canvas binary failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let checksum = String::from_utf8_lossy(&run.stdout)
        .trim()
        .parse::<u64>()
        .expect("numeric checksum");
    assert_ne!(checksum, 0);
    assert_eq!(checksum, 3_937_824_821_488_525_129);
}
