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
    let src = include_str!("../benchmarks/bench_01_tree.scadi");
    assert!(!src.contains("for "));
    assert!(src.contains("iterate "));
    let c = compile_pipeline(src);
    assert!(c.contains("sk_fs_list("));
    assert!(c.contains("strcmp(__when_tmp_"));
}

#[test]
fn showcase_read_stats_compiles() {
    let src = include_str!("../benchmarks/bench_02_read_stats.scadi");
    assert!(!src.contains("for "));
    let c = compile_pipeline(src);
    assert!(c.contains("sk_read_file("));
    assert!(c.contains("sk_text_find("));
}

#[test]
fn showcase_find_count_compiles() {
    let src = include_str!("../benchmarks/bench_03_find_count.scadi");
    assert!(!src.contains("for "));
    let c = compile_pipeline(src);
    assert!(c.contains("sk_text_slice("));
    assert!(c.contains("sk_text_find("));
}

#[test]
fn showcase_sum_ints_compiles() {
    let src = include_str!("../benchmarks/bench_04_sum_ints.scadi");
    assert!(!src.contains("for "));
    assert!(src.contains("iterate "));
    let c = compile_pipeline(src);
    assert!(c.contains("SkadiList_i64"));
    assert!(c.contains("sk_list_i64_push("));
}

#[test]
fn showcase_push_pop_compiles() {
    let src = include_str!("../benchmarks/bench_05_push_pop.scadi");
    assert!(!src.contains("for "));
    let c = compile_pipeline(src);
    assert!(c.contains("SkadiList_i64"));
    assert!(c.contains("sk_list_i64_pop("));
}
