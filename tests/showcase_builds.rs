use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

struct ShowcaseCase {
    name: &'static str,
    source: &'static str,
    extra_flags: &'static [&'static str],
}

const SHOWCASE_CASES: &[ShowcaseCase] = &[
    ShowcaseCase {
        name: "bench_01_tree",
        source: "benchmarks/bench_01_tree.skd",
        extra_flags: &[],
    },
    ShowcaseCase {
        name: "bench_02_read_stats",
        source: "benchmarks/bench_02_read_stats.skd",
        extra_flags: &[],
    },
    ShowcaseCase {
        name: "bench_03_find_count",
        source: "benchmarks/bench_03_find_count.skd",
        extra_flags: &[],
    },
    ShowcaseCase {
        name: "bench_04_sum_ints",
        source: "benchmarks/bench_04_sum_ints.skd",
        extra_flags: &[],
    },
    ShowcaseCase {
        name: "bench_05_push_pop",
        source: "benchmarks/bench_05_push_pop.skd",
        extra_flags: &[],
    },
    ShowcaseCase {
        name: "bench_06_struct_account",
        source: "benchmarks/bench_06_struct_account.skd",
        extra_flags: &[],
    },
    ShowcaseCase {
        name: "bench_07_struct_list",
        source: "benchmarks/bench_07_struct_list.skd",
        extra_flags: &[],
    },
    ShowcaseCase {
        name: "bench_08_path_list_helpers",
        source: "benchmarks/bench_08_path_list_helpers.skd",
        extra_flags: &[],
    },
    ShowcaseCase {
        name: "bench_09_math_navigation",
        source: "benchmarks/bench_09_math_navigation.skd",
        extra_flags: &["-lm"],
    },
    ShowcaseCase {
        name: "bench_10_v1_1_toolbox",
        source: "benchmarks/bench_10_v1_1_toolbox.skd",
        extra_flags: &["-lm"],
    },
    ShowcaseCase {
        name: "bench_11_task_channel_pipeline",
        source: "benchmarks/bench_11_task_channel_pipeline.skd",
        extra_flags: &["-pthread"],
    },
    ShowcaseCase {
        name: "bench_12_systems_pipeline",
        source: "benchmarks/bench_12_systems_pipeline.skd",
        extra_flags: &["-pthread", "-lm"],
    },
    ShowcaseCase {
        name: "bench_13_time_budget",
        source: "benchmarks/bench_13_time_budget.skd",
        extra_flags: &["-pthread"],
    },
];

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

fn compile_skadi_to_c(src: &str) -> String {
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    transpile_program_to_c(&program)
}

fn temp_artifact_paths(stem: &str) -> (PathBuf, PathBuf) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path = std::env::temp_dir();
    c_path.push(format!("{stem}_{stamp}.c"));
    let mut exe_path = std::env::temp_dir();
    exe_path.push(format!("{stem}_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }
    (c_path, exe_path)
}

fn compile_c_to_binary(compiler: &str, c_src: &str, stem: &str, extra_flags: &[&str]) {
    let (c_path, exe_path) = temp_artifact_paths(stem);
    fs::write(&c_path, c_src).expect("write C source");

    let mut compile_cmd = Command::new(compiler);
    compile_cmd.arg(&c_path).arg("-o").arg(&exe_path);
    for flag in extra_flags {
        if cfg!(windows) && *flag == "-pthread" {
            continue;
        }
        compile_cmd.arg(flag);
    }

    let compile = compile_cmd.output().expect("run C compiler");
    if let Err(err) = fs::remove_file(&c_path) {
        eprintln!("cleanup warning for {:?}: {}", c_path, err);
    }
    if let Err(err) = fs::remove_file(&exe_path)
        && exe_path.exists()
    {
        eprintln!("cleanup warning for {:?}: {}", exe_path, err);
    }

    assert!(
        compile.status.success(),
        "C compile failed for {stem}: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
}

fn load_showcase_source(rel_path: &str) -> String {
    let manifest_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let full_path = manifest_root.join(rel_path);
    fs::read_to_string(&full_path).unwrap_or_else(|err| {
        panic!(
            "failed to read showcase source '{}': {}",
            full_path.display(),
            err
        )
    })
}

#[test]
fn all_showcases_build_to_native_binaries() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping showcase build suite: no clang/gcc/cc in PATH.");
        return;
    };

    for case in SHOWCASE_CASES {
        let src = load_showcase_source(case.source);
        let c = compile_skadi_to_c(&src);
        compile_c_to_binary(compiler, &c, case.name, case.extra_flags);
    }
}
