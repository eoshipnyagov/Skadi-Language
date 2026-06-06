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
    assert!(c.contains("int main(int argc, char **argv) {"));
    assert!(c.contains("cli_args = sk_args(argc, argv);"));
    assert!(c.contains("sk_fs_list("));
    assert!(c.contains("walk(full, dirs_only, max_depth, (current_depth + 1));"));
    assert!(c.contains("strcmp(__when_tmp_"));
}

#[test]
fn showcase_read_stats_compiles() {
    let src = include_str!("../benchmarks/bench_02_read_stats.skd");
    assert!(!src.contains("for "));
    let c = compile_pipeline(src);
    assert!(c.contains("int main(int argc, char **argv) {"));
    assert!(c.contains("cli_args = sk_args(argc, argv);"));
    assert!(c.contains("sk_read_file("));
    assert!(c.contains("sk_text_slice(data, start, n)"));
    assert!(c.contains("sk_text_find("));
    assert!(c.contains("sk_output_text(sk_text_concat(\"file: \", path));"));
}

#[test]
fn showcase_find_count_compiles() {
    let src = include_str!("../benchmarks/bench_03_find_count.skd");
    assert!(!src.contains("for "));
    let c = compile_pipeline(src);
    assert!(c.contains("int main(int argc, char **argv) {"));
    assert!(c.contains("needle = sk_list_text_get(&cli_args, (i + 1));"));
    assert!(c.contains("const char* data = sk_read_file(path);"));
    assert!(c.contains("sk_text_slice("));
    assert!(c.contains("sk_text_find("));
    assert!(c.contains("sk_output_text(sk_text_concat(\"needle: \", needle));"));
}

#[test]
fn showcase_sum_ints_compiles() {
    let src = include_str!("../benchmarks/bench_04_sum_ints.skd");
    assert!(!src.contains("for "));
    assert!(src.contains("iterate "));
    let c = compile_pipeline(src);
    assert!(c.contains("int main(int argc, char **argv) {"));
    assert!(c.contains("SkadiList_i64"));
    assert!(c.contains("sk_list_i64_push("));
    assert!(c.contains("for (size_t __i = 0; __i < xs.len; ++__i) {"));
    assert!(c.contains("sk_output_int(sum);"));
}

#[test]
fn showcase_push_pop_compiles() {
    let src = include_str!("../benchmarks/bench_05_push_pop.skd");
    assert!(!src.contains("for "));
    let c = compile_pipeline(src);
    assert!(c.contains("int main(int argc, char **argv) {"));
    assert!(c.contains("SkadiList_i64"));
    assert!(c.contains("sk_list_i64_pop("));
    assert!(c.contains("if (sk_list_i64_pop(&stack, &value) != 0) {"));
    assert!(c.contains("j = n;"));
    assert!(c.contains("(i % 1024)"));
}

#[test]
fn showcase_struct_account_compiles() {
    let src = include_str!("../benchmarks/bench_06_struct_account.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("} Account;"));
    assert!(c.contains("int64_t Account_withdraw(Account *my, int64_t amount)"));
    assert!(c.contains("Account_deposit(&acc, 25)"));
    assert!(c.contains("Account_snapshot(&acc)"));
    assert!(c.contains("my->balance = (my->balance - amount);"));
}

#[test]
fn showcase_struct_list_compiles() {
    let src = include_str!("../benchmarks/bench_07_struct_list.skd");
    assert!(src.contains("iterate "));
    let c = compile_pipeline(src);
    assert!(c.contains("SkadiList_Sensor"));
    assert!(c.contains("sk_list_Sensor_push("));
    assert!(c.contains("Sensor_is_hot(&s, 30)"));
}

#[test]
fn showcase_path_list_helpers_compiles() {
    let src = include_str!("../benchmarks/bench_08_path_list_helpers.skd");
    assert!(src.contains("iterate "));
    let c = compile_pipeline(src);
    assert!(c.contains("int64_t skadi_user_main()"));
    assert!(c.contains("skadi_user_main();"));
    assert!(c.contains("sk_fs_list("));
    assert!(c.contains("sk_fs_join("));
    assert!(c.contains("sk_fs_is_dir("));
}

#[test]
fn showcase_math_navigation_compiles() {
    let src = include_str!("../benchmarks/bench_09_math_navigation.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("#include <math.h>"));
    assert!(c.contains("double heading_rad = ((heading_deg * M_PI) / 180.0);"));
    assert!(c.contains("cos("));
    assert!(c.contains("sin("));
    assert!(c.contains("atan2("));
    assert!(c.contains(
        "double bounded = ((restored_deg < 0) ? 0 : ((restored_deg > 90) ? 90 : restored_deg));"
    ));
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
    assert!(c.contains("sk_free_text((void*)summary);"));
}
