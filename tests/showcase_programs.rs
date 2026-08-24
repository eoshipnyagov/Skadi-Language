use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

fn compile_pipeline(src: &str) -> String {
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let warnings = semantic_style_warnings(&program);
    assert!(warnings.is_empty(), "showcase style warnings: {warnings:?}");
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
    assert!(c.contains("walk(full, dirs_only, max_depth, sk_num_add_int(current_depth, 1));"));
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
    assert!(c.contains("needle = sk_list_text_get(&cli_args, sk_num_add_int(i, 1));"));
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
    assert!(c.contains("SkadiList_int stack"));
    assert!(c.contains("sk_list_int_pop("));
    assert!(c.contains("if (sk_list_int_pop(&stack, &value) != 0) {"));
    assert!(c.contains("j = n;"));
    assert!(c.contains("sk_num_mod_int(i, 1024)"));
}

#[test]
fn showcase_struct_account_compiles() {
    let src = include_str!("../benchmarks/bench_06_struct_account.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("} Account;"));
    assert!(c.contains("SkInt Account_withdraw(Account *my, SkInt amount)"));
    assert!(c.contains("Account_deposit(&acc, 25)"));
    assert!(c.contains("Account_snapshot(&acc)"));
    assert!(c.contains("my->balance = sk_num_sub_int(my->balance, amount);"));
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
    assert!(c.contains("SkInt skadi_user_main()"));
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
    assert!(c.contains("float heading = 0.785398185f;"));
    assert!(c.contains("cosf("));
    assert!(c.contains("sinf("));
    assert!(c.contains("atan2f("));
    assert!(c.contains("float bounded = sk_math_clamp_float(restored_deg, 0, 90);"));
}

#[test]
fn showcase_v1_1_toolbox_compiles() {
    let src = include_str!("../benchmarks/bench_10_v1_1_toolbox.skd");
    assert!(!src.contains("for "));
    assert!(src.contains("iterate "));
    assert!(src.contains("danger fn"));
    let c = compile_pipeline(src);
    assert!(c.contains("typedef enum ErrorCode"));
    assert!(c.contains("int safe_speed(float distance, float seconds, float *out)"));
    assert!(c.contains("SkadiList_Waypoint"));
    assert!(c.contains("Waypoint_distance_from_origin(&point)"));
    assert!(c.contains("if (safe_speed(total, 0, &fallback) != 0) {"));
    assert!(c.contains("sk_list_Waypoint_free(&route);"));
    assert!(c.contains("sk_free_text((void*)summary);"));
}

#[test]
fn showcase_task_channel_pipeline_compiles() {
    let src = include_str!("../benchmarks/bench_11_task_channel_pipeline.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("sk_channel_create(1, sizeof(Reading))"));
    assert!(c.contains("sk_task_start(&producer_task"));
    assert!(c.contains("sk_channel_receive_Reading"));
}

#[test]
fn showcase_systems_pipeline_compiles() {
    let src = include_str!("../benchmarks/bench_12_systems_pipeline.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("SkMemoryRegion report_memory_storage"));
    assert!(c.contains("static SK_THREAD_LOCAL SkMemoryRegion *sk_active_region"));
    assert!(c.contains("static SK_THREAD_LOCAL SkTask *sk_current_task"));
    assert!(c.contains("sk_channel_create(1, sizeof(Reading))"));
    assert!(c.contains("sk_mem_set_active(report_memory)"));
}

#[test]
fn showcase_time_budget_compiles() {
    let src = include_str!("../benchmarks/bench_13_time_budget.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("int64_t measure_step(int64_t work_budget)"));
    assert!(c.contains("sk_time_sleep(work_budget)"));
    assert!(c.contains("sk_time_elapsed(started_at)"));
    assert!(c.contains("sk_task_start(&measurement_task"));
}

#[test]
fn showcase_byte_size_budget_compiles() {
    let src = include_str!("../benchmarks/bench_14_byte_size_budget.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("int64_t calculate_capacity(int64_t payload)"));
    assert!(c.contains("SkadiList_bytesize checkpoints"));
    assert!(c.contains("int64_t buffer_memory_capacity = capacity"));
    assert!(c.contains("buffer_memory_capacity <= 0 || !sk_mem_region_init"));
}

#[test]
fn showcase_angle_navigation_compiles() {
    let src = include_str!("../benchmarks/bench_15_angle_navigation.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("float steer(float heading, float correction, float influence)"));
    assert!(c.contains("SkadiList_angle checkpoints"));
    assert!(c.contains("float measured = atan2f(y, x)"));
    assert!(c.contains("cosf(target)"));
    assert!(c.contains("sinf(target)"));
}

#[test]
fn showcase_vector_navigation_compiles() {
    let src = include_str!("../benchmarks/bench_16_vector_navigation.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("Vec2 direction(Vec2 from, Vec2 to)"));
    assert!(c.contains("sk_vec2_normalize(sk_vec2_sub(to, from))"));
    assert!(c.contains("sk_vec3_cross(east, north)"));
    assert!(c.contains("sk_vec4_normalize(weights)"));
}

#[test]
fn showcase_canvas_palette_compiles() {
    let src = include_str!("../benchmarks/bench_17_canvas_palette.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("sk_color_hex(\"#1d1f21\", 255)"));
    assert!(c.contains("Color_terminal_bright_yellow"));
    assert!(c.contains("sk_canvas_fill_rect(&frame"));
    assert!(c.contains("sk_canvas_circle(&frame"));
    assert!(c.contains("sk_canvas_checksum(&frame)"));
    assert!(!c.contains("StretchDIBits"));
}

#[test]
fn showcase_bit_registers_compiles() {
    let src = include_str!("../benchmarks/bench_18_bit_registers.skd");
    let c = compile_pipeline(src);
    assert!(c.contains("uint8_t control = 1"));
    assert!(c.contains("sk_bit_set_u8(control, 3)"));
    assert!(c.contains("sk_bit_or_u16(status, 52)"));
    assert!(c.contains("sk_bit_shift_right_i8(signed_pattern, 1)"));
    assert!(c.contains("SC-RT-340"));
}
