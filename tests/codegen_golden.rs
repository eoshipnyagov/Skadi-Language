use std::fs;

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn normalize(s: &str) -> String {
    let mut normalized = s.replace("\r\n", "\n");
    if let Some(runtime_start) = normalized.find("static void sk_numeric_panic") {
        let runtime_tail = &normalized[runtime_start..];
        let last_helper = runtime_tail
            .find("static uint64_t sk_num_pow_u64")
            .expect("numeric runtime should contain its final helper");
        let helper_tail = &runtime_tail[last_helper..];
        let runtime_end = helper_tail
            .find("\n\n")
            .expect("numeric runtime should end with a blank line");
        let absolute_end = runtime_start + last_helper + runtime_end + 2;
        normalized.replace_range(runtime_start..absolute_end, "");
        normalized = normalized
            .replace("#include <stddef.h>\n#include <stdlib.h>\n", "")
            .replace("#include <limits.h>\n\n", "");
    }
    normalized.trim().to_string()
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
