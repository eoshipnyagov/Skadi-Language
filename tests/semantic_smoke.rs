use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

#[test]
fn semantic_passes_for_defined_variables() {
    let src = "a = 1\nb = a + 2\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_fails_for_use_before_definition() {
    let src = "b = a + 2\na = 1\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("Use-before-definition"));
    assert!(err.contains("a"));
}

#[test]
fn semantic_fails_for_redeclaration_in_same_scope() {
    let src = "a = 1\na = 2\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("Redeclaration in same scope"));
    assert!(err.contains("a"));
}

#[test]
fn semantic_fails_for_self_reference_on_initialization() {
    let src = "x = x + 1\n";
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
    x = 1
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
    x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}
