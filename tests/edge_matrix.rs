use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn pipeline_ok(src: &str) -> String {
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    transpile_program_to_c(&program)
}

fn semantic_err(src: &str) -> String {
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect_err("semantic should fail")
}

#[test]
fn edge_numeric_lists_cover_all_width_families_codegen_shape() {
    let src = r#"
new i8 List a = [1, 2]
new i16 List b = [1, 2]
new i32 List c = [1, 2]
new i64 List d = [1, 2]
new u8 List e = [1, 2]
new u16 List f = [1, 2]
new u32 List g = [1, 2]
new u64 List h = [1, 2]
new f32 List i = [1.0, 2.0]
new f64 List j = [1.0, 2.0]
new bool List k = [true, false]
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("SkadiList_i8 a = sk_list_i8_new();"));
    assert!(c.contains("SkadiList_i16 b = sk_list_i16_new();"));
    assert!(c.contains("SkadiList_i32 c = sk_list_i32_new();"));
    assert!(c.contains("SkadiList_i64 d = sk_list_i64_new();"));
    assert!(c.contains("SkadiList_u8 e = sk_list_u8_new();"));
    assert!(c.contains("SkadiList_u16 f = sk_list_u16_new();"));
    assert!(c.contains("SkadiList_u32 g = sk_list_u32_new();"));
    assert!(c.contains("SkadiList_u64 h = sk_list_u64_new();"));
    assert!(c.contains("SkadiList_f32 i = sk_list_f32_new();"));
    assert!(c.contains("SkadiList_f64 j = sk_list_f64_new();"));
    assert!(c.contains("SkadiList_bool k = sk_list_bool_new();"));
}

#[test]
fn edge_path_list_maps_to_text_runtime_helpers() {
    let src = r#"
new Path List entries = fs.list(".")
new Path p = entries[0]
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("SkadiList_text entries = sk_list_text_new();"));
    assert!(c.contains("entries = sk_fs_list(\".\");"));
    assert!(c.contains("const char* p = sk_list_text_get(&entries, 0);"));
}

#[test]
fn edge_text_index_and_slice_extreme_bounds_codegen_shape() {
    let src = r#"
new Text t = "abc"
new char c0 = t[-1]
new char c1 = t[99]
new Text s0 = slice(t, -100, 100)
new Text s1 = slice(t, 10, 2)
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("char c0 = sk_text_char_at(t, (-1));"));
    assert!(c.contains("char c1 = sk_text_char_at(t, 99);"));
    assert!(c.contains("const char* s0 = sk_text_slice(t, (-100), 100);"));
    assert!(c.contains("const char* s1 = sk_text_slice(t, 10, 2);"));
}

#[test]
fn edge_text_utf8_contract_is_byte_oriented_shape() {
    let src = r#"
new Text t = "Пр"
new Int n = len(t)
new char b0 = t[0]
new Text s = slice(t, 0, 2)
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("int64_t n = ((int64_t)strlen(t));"));
    assert!(c.contains("char b0 = sk_text_char_at(t, 0);"));
    assert!(c.contains("const char* s = sk_text_slice(t, 0, 2);"));
}

#[test]
fn edge_rejects_fs_join_argument_types() {
    let src = r#"
new Int x = 1
new Text p = fs.join(x, "src")
"#;
    let err = semantic_err(src);
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("builtin 'fs.join' expects (Text, Text)"));
}

#[test]
fn edge_rejects_write_argument_types() {
    let src = r#"
new Int x = 1
new Int y = write(x, "hello")
"#;
    let err = semantic_err(src);
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("builtin 'write' expects (Text, Text)"));
}

#[test]
fn edge_rejects_args_with_unexpected_arguments() {
    let src = r#"
new Text List xs = args(1)
"#;
    let err = semantic_err(src);
    assert!(err.contains("SC-SEM-033"));
    assert!(err.contains("builtin 'args' expects 0 arguments"));
}

#[test]
fn edge_struct_list_method_iteration_shape() {
    let src = r#"
struct Counter {
    Int value
    fn bump(Int d) Int {
        my.value = my.value + d
        return my.value
    }
}

new Counter List xs = []
xs.push({value = 1})
xs.push({value = 3})
new Int sum = 0
iterate xs as item {
    sum = sum + item.bump(2)
}
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("SkadiList_Counter xs = sk_list_Counter_new();"));
    assert!(c.contains("Counter_bump(&item, 2)"));
}

#[test]
fn edge_danger_on_error_flow_with_explicit_errorcode() {
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

new Int x = 0
x = div_safe(10, 0) on error {
    x = -1
}
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("return ErrorCode_ZeroDivision;"));
    assert!(c.contains("if (div_safe(10, 0, &x) != 0) {"));
}
