use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use v01::codegen::{ensure_codegen_supported, transpile_program_to_c};
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

fn example_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn read_example(rel: &str) -> String {
    fs::read_to_string(example_path(rel)).expect("example file should be readable")
}

fn parse_ok(rel: &str) -> v01::ast_nodes::Program {
    let src = read_example(rel);
    let tokens = lex(&src).expect("lex should succeed");
    parse_program(&tokens).expect("parse should succeed")
}

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

fn compile_c_and_execute(
    compiler: &str,
    c_src: &str,
    stem: &str,
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
    if !cfg!(windows) {
        compile_cmd.arg("-pthread");
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

fn compile_program_to_c(src: &str) -> String {
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    transpile_program_to_c(&program)
}

#[test]
fn memory_active_region_is_thread_local_at_runtime() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping memory TLS runtime test: no clang/gcc/cc in PATH.");
        return;
    };

    let mut c_src = compile_program_to_c(
        r#"
Memory bootstrap_memory = memory(64b)
bootstrap_memory.clear()
"#,
    );
    c_src = c_src.replacen("int main(void)", "static int skadi_original_main(void)", 1);
    c_src.push_str(
        r#"

#if defined(_WIN32)
#include <windows.h>

static volatile LONG sk_tls_ready = 0;

static DWORD WINAPI sk_tls_worker(LPVOID unused) {
    (void)unused;
    SkMemoryRegion region;
    if (!sk_mem_region_init(&region, 128)) return 2;
    sk_mem_set_active(&region);
    InterlockedIncrement(&sk_tls_ready);
    while (InterlockedCompareExchange(&sk_tls_ready, 0, 0) < 2) Sleep(0);
    DWORD result = sk_mem_current() == &region ? 0 : 3;
    sk_mem_set_active(NULL);
    free(region.buffer);
    return result;
}

int main(void) {
    HANDLE first = CreateThread(NULL, 0, sk_tls_worker, NULL, 0, NULL);
    HANDLE second = CreateThread(NULL, 0, sk_tls_worker, NULL, 0, NULL);
    if (!first || !second) return 4;
    WaitForSingleObject(first, INFINITE);
    WaitForSingleObject(second, INFINITE);
    DWORD first_result = 0;
    DWORD second_result = 0;
    GetExitCodeThread(first, &first_result);
    GetExitCodeThread(second, &second_result);
    CloseHandle(first);
    CloseHandle(second);
    return first_result == 0 && second_result == 0 ? 0 : 5;
}
#else
#include <pthread.h>

static pthread_mutex_t sk_tls_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t sk_tls_ready_condition = PTHREAD_COND_INITIALIZER;
static int sk_tls_ready = 0;

static void* sk_tls_worker(void *unused) {
    (void)unused;
    SkMemoryRegion region;
    if (!sk_mem_region_init(&region, 128)) return (void*)2;
    sk_mem_set_active(&region);

    pthread_mutex_lock(&sk_tls_lock);
    sk_tls_ready += 1;
    if (sk_tls_ready == 2) {
        pthread_cond_broadcast(&sk_tls_ready_condition);
    } else {
        while (sk_tls_ready < 2) {
            pthread_cond_wait(&sk_tls_ready_condition, &sk_tls_lock);
        }
    }
    pthread_mutex_unlock(&sk_tls_lock);

    void *result = sk_mem_current() == &region ? 0 : (void*)3;
    sk_mem_set_active(NULL);
    free(region.buffer);
    return result;
}

int main(void) {
    pthread_t first;
    pthread_t second;
    if (pthread_create(&first, NULL, sk_tls_worker, NULL) != 0) return 4;
    if (pthread_create(&second, NULL, sk_tls_worker, NULL) != 0) return 4;
    void *first_result = NULL;
    void *second_result = NULL;
    pthread_join(first, &first_result);
    pthread_join(second, &second_result);
    return first_result == NULL && second_result == NULL ? 0 : 5;
}
#endif
"#,
    );

    let run = compile_c_and_execute(compiler, &c_src, "memory_tls_runtime", &[], None);
    assert!(
        run.status.success(),
        "thread-local active Memory isolation failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn positive_memory_examples_pass_frontend_and_codegen() {
    let examples = [
        "examples/memory/positive/01_loaded_text_asset.skd",
        "examples/memory/positive/02_local_scratch_preview.skd",
        "examples/memory/positive/03_sensor_batch_external_memory.skd",
        "examples/memory/positive/04_explicit_recovery.skd",
        "examples/memory/positive/05_nested_scratch_inside_result_region.skd",
    ];

    for rel in examples {
        let program = parse_ok(rel);
        semantic_analyze(&program).unwrap_or_else(|err| {
            panic!("semantic analysis should pass for {rel}: {err}");
        });
        ensure_codegen_supported(&program)
            .unwrap_or_else(|err| panic!("memory backend support should pass for {rel}: {err}"));
        let c_src = transpile_program_to_c(&program);
        assert!(
            c_src.contains("SkMemoryRegion"),
            "expected memory runtime in generated C for {rel}"
        );
    }
}

#[test]
fn positive_memory_examples_build_to_native_binaries() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping memory native build test: no clang/gcc/cc in PATH.");
        return;
    };

    let examples = [
        (
            "mem_positive_01",
            "examples/memory/positive/01_loaded_text_asset.skd",
        ),
        (
            "mem_positive_02",
            "examples/memory/positive/02_local_scratch_preview.skd",
        ),
        (
            "mem_positive_03",
            "examples/memory/positive/03_sensor_batch_external_memory.skd",
        ),
        (
            "mem_positive_04",
            "examples/memory/positive/04_explicit_recovery.skd",
        ),
        (
            "mem_positive_05",
            "examples/memory/positive/05_nested_scratch_inside_result_region.skd",
        ),
    ];

    for (stem, rel) in examples {
        let program = parse_ok(rel);
        semantic_analyze(&program).expect("semantic analysis should pass");
        let c_src = transpile_program_to_c(&program);
        let run = compile_c_and_execute(compiler, &c_src, stem, &[], None);
        assert!(
            run.status.success(),
            "native binary should run successfully for {rel}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
}

#[test]
fn large_memory_example_builds_and_runs() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping large memory example runtime test: no clang/gcc/cc in PATH.");
        return;
    };

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let program = parse_ok("examples/memory/large/01_log_analyzer.skd");
    semantic_analyze(&program).expect("semantic analysis should pass");
    ensure_codegen_supported(&program).expect("memory backend support should pass");
    let c_src = transpile_program_to_c(&program);
    let run = compile_c_and_execute(compiler, &c_src, "memory_large_01", &[], Some(&repo_root));

    assert!(run.status.success(), "large memory example should run");

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("preview status"));
    assert!(stdout.contains("preview contains service"));
    assert!(stdout.contains("true"));
    assert!(stdout.contains("total lines"));
    assert!(stdout.contains("8"));
    assert!(stdout.contains("error lines"));
    assert!(stdout.contains("warning lines"));
    assert!(stdout.contains("todo lines"));
    assert!(stdout.contains("alert lines"));
    assert!(stdout.contains("6"));
    assert!(stdout.contains("WARN cache warmup slow"));
    assert!(stdout.contains("ERROR failed to open config"));
    assert!(stdout.contains("TODO review alert thresholds"));
}

#[test]
fn memory_runtime_examples_build_and_run() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping memory runtime e2e test: no clang/gcc/cc in PATH.");
        return;
    };

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let external_memory_return = r#"
struct LoadedText {
    Text content
}

fn load_text(Memory assets_memory, Path path) LoadedText {
    place in assets_memory {
        new Text file_text = read(path)
        new LoadedText result = {content = file_text}
        return result
    }
}

Memory assets_memory = memory(4kb)
new LoadedText loaded = load_text(assets_memory, "benchmarks/showcase-data/sample_weather.txt")
output(contains(loaded.content, "temperature"))
"#;
    let run = compile_c_and_execute(
        compiler,
        &compile_program_to_c(external_memory_return),
        "memory_runtime_return",
        &[],
        Some(&repo_root),
    );
    assert!(run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("true"),
        "expected successful loaded text output, got: {}",
        String::from_utf8_lossy(&run.stdout)
    );

    let local_scratch = r#"
fn preview_file(Path path) Int {
    Memory scratch_memory = memory(64kb) on error {
        return 1
    }

    place in scratch_memory {
        new Text file_text = read(path)
        output(contains(file_text, "temperature"))
    } on error {
        scratch_memory.clear()
        return 2
    }

    scratch_memory.clear()
    return 0
}

output(preview_file("benchmarks/showcase-data/sample_weather.txt"))
"#;
    let run = compile_c_and_execute(
        compiler,
        &compile_program_to_c(local_scratch),
        "memory_runtime_scratch",
        &[],
        Some(&repo_root),
    );
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("true"));
    assert!(stdout.contains("0"));

    let list_payload = r#"
fn collect_batch(Memory frame_memory) Int List {
    place in frame_memory {
        new Int List reading_values = [10, 20, 30]
        return reading_values
    }
}

Memory frame_memory = memory(4kb)
new Int List batch = collect_batch(frame_memory)
output(len(batch))
"#;
    let run = compile_c_and_execute(
        compiler,
        &compile_program_to_c(list_payload),
        "memory_runtime_list",
        &[],
        None,
    );
    assert!(run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("3"),
        "expected list length output, got: {}",
        String::from_utf8_lossy(&run.stdout)
    );

    let overflow_recovery = r#"
struct ConfigText {
    Text content
    Bool fallback_used
}

fn load_config(Memory assets_memory, Path path) ConfigText {
    place in assets_memory {
        new Text raw_text = read(path)
        new ConfigText config_result = {content = raw_text, fallback_used = false}
        return config_result
    } on error {
        assets_memory.clear()
        return {content = "memory overflow", fallback_used = true}
    }
}

Memory assets_memory = memory(16b)
new ConfigText cfg = load_config(assets_memory, "benchmarks/showcase-data/sample_weather.txt")
output(cfg.fallback_used == true)
"#;
    let run = compile_c_and_execute(
        compiler,
        &compile_program_to_c(overflow_recovery),
        "memory_runtime_overflow",
        &[],
        Some(&repo_root),
    );
    assert!(run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("true"),
        "expected overflow recovery output, got: {}",
        String::from_utf8_lossy(&run.stdout)
    );

    let nested_regions = r#"
struct LoadedSnippet {
    Text content
}

fn load_with_preview(Memory assets_memory, Memory scratch_memory, Path path) LoadedSnippet {
    place in assets_memory {
        place in scratch_memory {
            new Text preview_text = read(path)
            new Text head_text = slice(preview_text, 0, 16)
            output(head_text)
        } on error {
            scratch_memory.clear()
            output("scratch overflow")
        }

        new Text result_text = read(path)
        new LoadedSnippet result = {content = result_text}
        return result
    } on error {
        assets_memory.clear()
        return {content = "outer overflow"}
    }
}

Memory assets_memory = memory(8kb)
Memory scratch_memory = memory(16b)
new LoadedSnippet loaded = load_with_preview(assets_memory, scratch_memory, "benchmarks/showcase-data/sample_weather.txt")
output("nested ok")
output(contains(loaded.content, "temperature"))
"#;
    let run = compile_c_and_execute(
        compiler,
        &compile_program_to_c(nested_regions),
        "memory_runtime_nested_regions",
        &[],
        Some(&repo_root),
    );
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("scratch overflow"));
    assert!(stdout.contains("nested ok"));
    assert!(stdout.contains("true"));
}

#[test]
fn memory_style_pitfall_example_emits_collapsed_name_warning() {
    let program = parse_ok("examples/memory/pitfalls/01_collapsed_field_names.skd");
    semantic_analyze(&program).expect("semantic analysis should pass");
    let warnings = semantic_style_warnings(&program);
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("avoid collapsed field init")),
        "expected collapsed field init warning, got: {warnings:?}"
    );
}

#[test]
fn negative_memory_examples_fail_with_expected_diagnostics() {
    let examples = [
        (
            "examples/memory/negative/01_local_memory_return_escape.skd",
            "SC-SEM-061",
            "local Memory",
        ),
        (
            "examples/memory/negative/02_in_block_clear.skd",
            "SC-SEM-060",
            "forbidden in-block clear",
        ),
        (
            "examples/memory/negative/03_memory_in_struct.skd",
            "SC-SEM-062",
            "struct field type",
        ),
        (
            "examples/memory/negative/04_memory_list.skd",
            "SC-SEM-062",
            "variable declaration type",
        ),
        (
            "examples/memory/negative/05_memory_copy_assignment.skd",
            "SC-SEM-062",
            "cannot be reassigned or copied",
        ),
        (
            "examples/memory/negative/06_use_after_clear.skd",
            "SC-SEM-061",
            "use-after-clear",
        ),
        (
            "examples/memory/negative/07_store_into_longer_lived_owner.skd",
            "SC-SEM-060",
            "longer-lived owner",
        ),
        (
            "examples/memory/negative/08_nested_same_memory_place_in.skd",
            "SC-SEM-060",
            "nested place in same Memory is forbidden",
        ),
    ];

    for (rel, code, marker) in examples {
        let program = parse_ok(rel);
        let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
        assert!(err.contains(code), "expected {code} for {rel}, got: {err}");
        assert!(
            err.contains(marker),
            "expected marker '{marker}' for {rel}, got: {err}"
        );
    }
}
