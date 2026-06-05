use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn compile_pipeline(src: &str) -> String {
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    transpile_program_to_c(&program)
}

#[test]
fn showcase_tree_compiles() {
    let src = include_str!("../benchmarks/bench_01_tree.skd");
    assert!(!src.contains("for "));
    assert!(src.contains("iterate "));
    let c = compile_pipeline(src);
    assert!(c.contains("sk_fs_list("));
    assert!(c.contains("strcmp(__when_tmp_"));
}

#[test]
fn showcase_read_stats_compiles() {
    let src = include_str!("../benchmarks/bench_02_read_stats.skd");
    assert!(!src.contains("for "));
    let c = compile_pipeline(src);
    assert!(c.contains("sk_read_file("));
    assert!(c.contains("sk_text_find("));
}

#[test]
fn showcase_find_count_compiles() {
    let src = include_str!("../benchmarks/bench_03_find_count.skd");
    assert!(!src.contains("for "));
    let c = compile_pipeline(src);
    assert!(c.contains("sk_text_slice("));
    assert!(c.contains("sk_text_find("));
}

#[test]
fn showcase_sum_ints_compiles() {
    let src = include_str!("../benchmarks/bench_04_sum_ints.skd");
    assert!(!src.contains("for "));
    assert!(src.contains("iterate "));
    let c = compile_pipeline(src);
    assert!(c.contains("SkadiList_i64"));
    assert!(c.contains("sk_list_i64_push("));
}

#[test]
fn showcase_push_pop_compiles() {
    let src = include_str!("../benchmarks/bench_05_push_pop.skd");
    assert!(!src.contains("for "));
    let c = compile_pipeline(src);
    assert!(c.contains("SkadiList_i64"));
    assert!(c.contains("sk_list_i64_pop("));
}

#[test]
fn showcase_math_navigation_compiles() {
    let src = include_str!("../benchmarks/bench_09_math_navigation.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("#include <math.h>"));
    assert!(c.contains("cos("));
    assert!(c.contains("sin("));
    assert!(c.contains("atan2("));
}

#[test]
fn showcase_v1_1_toolbox_compiles() {
    let src = include_str!("../benchmarks/bench_10_v1_1_toolbox.skd");
    assert!(!src.contains("for "));
    assert!(src.contains("iterate "));
    assert!(src.contains("danger fn"));
    let c = compile_pipeline(src);
    assert!(c.contains("typedef enum ErrorCode"));
    assert!(c.contains("int safe_speed(double distance, double seconds, double *out)"));
    assert!(c.contains("SkadiList_Waypoint"));
    assert!(c.contains("Waypoint_distance_from_origin(&point)"));
    assert!(c.contains("if (safe_speed(total, 0, &fallback) != 0) {"));
    assert!(c.contains("sk_list_Waypoint_free(&route);"));
    assert!(c.contains("free((void*)summary);"));
}
