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
fn conformance_control_flow_family() {
    let src = r#"
new Int x = 3
if x > 0 {
    x = x - 1
} else {
    x = 0
}
while x > 0 {
    x = x - 1
}
loop {
    x = x + 1
    return
}
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("if ("));
    assert!(c.contains("while ("));
}

#[test]
fn conformance_when_label_and_errorcode_family() {
    let src = r#"
label ErrorCode {
    Ok
    Invalid
}
danger fn parse_value(Int x) Int {
    if x < 0 {
        return error Invalid
    } else {
        return x
    }
}
new Int mode = 2
when mode {
    is 1 { new Int y = 10 }
    is 2, 3 { new Int y = 20 }
    else { new Int y = 0 }
}
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("typedef enum ErrorCode"));
    assert!(c.contains("return ErrorCode_Invalid;"));
    assert!(c.contains("else if ((__when_tmp_1 == 2) || (__when_tmp_1 == 3)) {"));
}

#[test]
fn conformance_list_family() {
    let src = r#"
new i32 List xs = [1, 2, 3]
new i32 current = 0
for item in xs {
    current = item
}
xs.push(4)
current = xs.pop() on error {
    current = -1
}
new Int n = len(xs)
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("SkadiList_i32"));
    assert!(c.contains("sk_list_i32_push(&xs, 4)"));
    assert!(c.contains("sk_list_i32_pop(&xs, &current)"));
}

#[test]
fn conformance_text_family() {
    let src = r#"
new Text t = "weather station"
new Int n = len(t)
new char ch = t[0]
new bool ok = contains(t, "station")
new Int idx = find(t, "ther")
new Text part = slice(t, 3, 7)
"#;
    let c = pipeline_ok(src);
    assert!(c.contains("strlen(t)"));
    assert!(c.contains("sk_text_find(t, \"ther\")"));
    assert!(c.contains("sk_text_slice(t, 3, 7)"));
}

#[test]
fn conformance_on_interrupt_and_struct_parse_semantic() {
    let src = r#"
struct Sensor {
    u8 address
}
on interrupt timer0 {
    new Int ticks = 1
}
"#;
    pipeline_ok(src);
}

#[test]
fn conformance_on_error_requires_danger_call() {
    let src = r#"
fn parse_value(Int x) Int {
    return x
}
new Int x = 1
x = parse_value(x) on error {
    x = 0
}
"#;
    let err = semantic_err(src);
    assert!(err.contains("on error requires danger fn call"));
}

#[test]
fn conformance_errorcode_must_start_with_ok() {
    let src = r#"
label ErrorCode {
    Invalid
    Ok
}
"#;
    let err = semantic_err(src);
    assert!(err.contains("must start with 'Ok'"));
}
