use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

#[test]
fn semantic_passes_for_defined_variables() {
    let src = "new a = 1\nnew b = a + 2\nb = b + 1\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_fails_for_use_before_definition() {
    let src = "b = a + 2\nnew a = 1\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("Use-before-definition"));
    assert!(err.contains("b"));
}

#[test]
fn semantic_fails_for_redeclaration_in_same_scope() {
    let src = "new a = 1\nnew a = 2\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("Redeclaration in same scope"));
    assert!(err.contains("a"));
}

#[test]
fn semantic_fails_for_self_reference_on_initialization() {
    let src = "new x = x + 1\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("Invalid initialization"));
    assert!(err.contains("x"));
}

#[test]
fn semantic_fails_for_on_error_without_danger_call_binding() {
    let src = r#"
on error {
    new x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("on error"));
}

#[test]
fn semantic_allows_on_interrupt_block() {
    let src = r#"
on interrupt timer0 {
    new x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_fails_for_undefined_variable_in_return() {
    let src = r#"
fn f() {
    return z
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("Use-before-definition"));
    assert!(err.contains("z"));
}

#[test]
fn semantic_allows_danger_on_error_assignment_for_defined_target() {
    let src = r#"
danger fn parse_value(bool x) Int {
    if x {
        return 1
    } else {
        return 0
    }
}

new bool x = true
x = parse_value(x) on error {
    x = false
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_on_error_for_non_danger_function() {
    let src = r#"
fn parse_value(bool x) Int {
    if x {
        return 1
    } else {
        return 0
    }
}

new bool x = true
x = parse_value(x) on error {
    x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("on error requires danger fn call"));
    assert!(err.contains("parse_value"));
}

#[test]
fn semantic_allows_return_error_with_errorcode_label() {
    let src = r#"
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn parse_value(bool x) Int {
    return error ZeroDivision
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_unknown_return_error_code() {
    let src = r#"
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn parse_value(bool x) Int {
    return error InvalidFormat
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("Unknown ErrorCode variant"));
}

#[test]
fn semantic_rejects_errorcode_without_ok_first() {
    let src = r#"
label ErrorCode {
    ZeroDivision
    Ok
}

danger fn parse_value(bool x) Int {
    return error ZeroDivision
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("must start with 'Ok'"));
}

#[test]
fn semantic_rejects_danger_fn_without_explicit_return() {
    let src = r#"
danger fn parse_value(bool x) Int {
    if x {
        return 1
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("must end with explicit return"));
}

#[test]
fn semantic_rejects_assignment_type_mismatch_bool_to_int() {
    let src = "new Int x = 1\nx = true\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("Type mismatch in assignment"));
}

#[test]
fn semantic_allows_int_to_float_widening() {
    let src = "new Float x = 1\nx = 2\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_non_bool_if_condition() {
    let src = r#"
new Int x = 1
if x {
    x = 2
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("if condition must be bool"));
}

#[test]
fn semantic_rejects_danger_on_error_arg_type_mismatch() {
    let src = r#"
danger fn parse_value(bool x) Int {
    if x {
        return 1
    } else {
        return 0
    }
}
new Int x = 1
x = parse_value(x) on error {
    x = 0
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("Argument type mismatch"));
}
