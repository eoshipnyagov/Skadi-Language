use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

#[test]
fn codegen_emits_main_and_assignment() {
    let src = "new x = 1 + 2\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int main(void)"));
    assert!(c.contains("int64_t x = (1 + 2);"));
}

#[test]
fn codegen_emits_function_signature() {
    let src = r#"
fn add(Int a, Int b) Int {
    new c = a + b
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t add(int64_t a, int64_t b)"));
}

#[test]
fn codegen_emits_control_flow_and_return() {
    let src = r#"
fn f(x) {
    if x {
        new y = 1
    } else {
        y = 2
    }
    while y {
        y = y - 1
    }
    return y
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("if (x) {"));
    assert!(c.contains("while (y) {"));
    assert!(c.contains("return y;"));
}

#[test]
fn codegen_respects_typed_new() {
    let src = "new Float temperature = 21.5\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("double temperature = 21.5;"));
}

#[test]
fn codegen_emits_danger_on_error_shape() {
    let src = r#"
new x = 0
x = parse_value(x) on error {
    x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("if (parse_value(x, &x) != 0) {"));
}

#[test]
fn codegen_emits_danger_fn_with_out_param() {
    let src = r#"
danger fn parse_value(Int x) Int {
    return x
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int parse_value(int64_t x, int64_t *out)"));
    assert!(c.contains("*out = x;"));
}

#[test]
fn codegen_emits_error_status_for_empty_return_in_danger_fn() {
    let src = r#"
danger fn parse_value(Int x) Int {
    return
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int parse_value(int64_t x, int64_t *out)"));
    assert!(c.contains("return 1;"));
}

#[test]
fn codegen_emits_error_enum_and_return_error() {
    let src = r#"
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn parse_value(Int x) Int {
    return error ZeroDivision
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("typedef enum ErrorCode"));
    assert!(c.contains("ErrorCode_ZeroDivision = 1"));
    assert!(c.contains("return ErrorCode_ZeroDivision;"));
}

#[test]
fn codegen_emits_regular_call_expression() {
    let src = r#"
fn add(Int a, Int b) Int {
    return a + b
}
new Int x = 1
new Int y = add(x, 2)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t y = add(x, 2);"));
}

#[test]
fn codegen_lowers_when_to_if_chain() {
    let src = r#"
new Int x = 2
when x {
    is 1 {
        new y = 10
    }
    is 2, 3 {
        new y = 20
    }
    else {
        new y = 0
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t __when_tmp_1 = x;"));
    assert!(c.contains("if ((__when_tmp_1 == 1)) {"));
    assert!(c.contains("else if ((__when_tmp_1 == 2) || (__when_tmp_1 == 3)) {"));
    assert!(c.contains("else {"));
}

#[test]
fn codegen_lowers_for_in_to_list_iteration_shape() {
    let src = r#"
new Int sum = 0
for item in items {
    sum = sum + item
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("for (size_t __i = 0; __i < items.len; ++__i) {"));
    assert!(c.contains("int64_t item = items.data[__i];"));
}

#[test]
fn codegen_lowers_for_in_with_typed_list_element() {
    let src = r#"
new u8 List items = [1, 2, 3]
for item in items {
    new Int x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("for (size_t __i = 0; __i < items.len; ++__i) {"));
    assert!(c.contains("uint8_t item = items.data[__i];"));
}

#[test]
fn codegen_emits_list_runtime_push_pop_shape() {
    let src = r#"
new i32 List xs = [1, 2]
new i32 x = 0
xs.push(3)
x = xs.pop() on error {
    x = 0
}
new Int n = len(xs)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("typedef struct {"));
    assert!(c.contains("SkadiList_i32 xs = sk_list_i32_new();"));
    assert!(c.contains("sk_list_i32_push(&xs, 3)"));
    assert!(c.contains("if (sk_list_i32_pop(&xs, &x) != 0) {"));
    assert!(c.contains("int64_t n = ((int64_t)xs.len);"));
}

#[test]
fn codegen_emits_list_runtime_for_multiple_scalar_types() {
    let src = r#"
new u8 List bu = [1, 2]
new f64 List fd = [1.0, 2.0]
new bool List bb = [true, false]
bu.push(3)
fd.push(3.5)
bb.push(true)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkadiList_u8 bu = sk_list_u8_new();"));
    assert!(c.contains("SkadiList_f64 fd = sk_list_f64_new();"));
    assert!(c.contains("SkadiList_bool bb = sk_list_bool_new();"));
    assert!(c.contains("sk_list_u8_push(&bu, 3)"));
    assert!(c.contains("sk_list_f64_push(&fd, 3.5)"));
    assert!(c.contains("sk_list_bool_push(&bb, true)"));
}

#[test]
fn codegen_emits_text_len_and_index_shape() {
    let src = r#"
new Text t = "weather"
new Int n = len(t)
new char c = t[0]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("const char* t = \"weather\";"));
    assert!(c.contains("int64_t n = ((int64_t)strlen(t));"));
    assert!(c.contains("char c = t[0];"));
}

#[test]
fn codegen_emits_text_contains_find_slice_shape() {
    let src = r#"
new Text t = "weather station"
new bool has = contains(t, "station")
new Int idx = find(t, "ther")
new Text tail = slice(t, 3, 7)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("bool has = (strstr(t, \"station\") != NULL);"));
    assert!(c.contains("int64_t idx = sk_text_find(t, \"ther\");"));
    assert!(c.contains("const char* tail = sk_text_slice(t, 3, 7);"));
}
