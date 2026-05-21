use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

#[test]
fn codegen_emits_main_and_assignment() {
    let src = "x = 1 + 2\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int main(void)"));
    assert!(c.contains("int x = (1 + 2);"));
}

#[test]
fn codegen_emits_function_signature() {
    let src = r#"
fn add(a, b) {
    c = a + b
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int add(int a, int b)"));
}
