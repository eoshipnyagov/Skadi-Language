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
fn program_list_for_len_and_push() {
    let src = r#"
new i32 List samples = [10, 20, 30]
new Int sum = 0
for item in samples {
    sum = sum + 1
}
samples.push(40)
new Int count = len(samples)
"#;
    let c = compile_pipeline(src);
    assert!(c.contains("SkadiList_i32 samples = sk_list_i32_new();"));
    assert!(c.contains("for (size_t __i = 0; __i < samples.len; ++__i) {"));
    assert!(c.contains("int32_t item = samples.data[__i];"));
    assert!(c.contains("sk_list_i32_push(&samples, 40)"));
    assert!(c.contains("int64_t count = ((int64_t)samples.len);"));
}

#[test]
fn program_list_pop_on_error_flow() {
    let src = r#"
new i32 List queue = [1]
new i32 current = 0
current = queue.pop() on error {
    current = -1
}
current = queue.pop() on error {
    current = -2
}
"#;
    let c = compile_pipeline(src);
    let pop_calls = c.matches("sk_list_i32_pop(&queue, &current)").count();
    assert_eq!(pop_calls, 2);
}

#[test]
fn program_when_and_labels_with_danger() {
    let src = r#"
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn safe_div(Int a, Int b) Int {
    if b == 0 {
        return error ZeroDivision
    } else {
        return a div b
    }
}

new Int mode = 2
when mode {
    is 1 {
        new Int x = 100
    }
    is 2, 3 {
        new Int x = 200
    }
    else {
        new Int x = 0
    }
}
"#;
    let c = compile_pipeline(src);
    assert!(c.contains("typedef enum ErrorCode"));
    assert!(c.contains("int safe_div(int64_t a, int64_t b, int64_t *out)"));
    assert!(c.contains("return ErrorCode_ZeroDivision;"));
    assert!(c.contains("else if ((__when_tmp_1 == 2) || (__when_tmp_1 == 3)) {"));
}

#[test]
fn program_iterate_as_alias_works_in_pipeline() {
    let src = r#"
new i32 List values = [1, 2, 3]
new Int total = 0
iterate values as v {
    total = total + 1
}
"#;
    let c = compile_pipeline(src);
    assert!(c.contains("for (size_t __i = 0; __i < values.len; ++__i) {"));
    assert!(c.contains("int32_t v = values.data[__i];"));
}
