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
