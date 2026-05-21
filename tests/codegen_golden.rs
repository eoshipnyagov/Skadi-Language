use std::fs;

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").trim().to_string()
}

fn assert_codegen_matches_fixture(skadi_path: &str, c_path: &str) {
    let skadi = fs::read_to_string(skadi_path).expect("read skadi fixture");
    let expected_c = fs::read_to_string(c_path).expect("read c fixture");

    let tokens = lex(&skadi).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");

    let actual_c = transpile_program_to_c(&program);
    assert_eq!(normalize(&actual_c), normalize(&expected_c));
}

#[test]
fn golden_codegen_simple_assignment() {
    assert_codegen_matches_fixture(
        "tests/fixtures/codegen_simple.skadi",
        "tests/fixtures/codegen_simple.c",
    );
}

#[test]
fn golden_codegen_function_and_top_level() {
    assert_codegen_matches_fixture(
        "tests/fixtures/codegen_function.skadi",
        "tests/fixtures/codegen_function.c",
    );
}
