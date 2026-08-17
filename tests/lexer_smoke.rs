use v01::lexer::lex;

#[test]
fn lex_error_reports_line_and_col_in_unified_style() {
    let src = "new x = 1\n@";
    let err = lex(src).expect_err("lex should fail on unexpected character");
    let rendered = err.to_string();
    assert!(rendered.contains("Lex error at line"));
    assert!(rendered.contains("col"));
    assert!(rendered.contains("unexpected character"));
}

#[test]
fn tokens_preserve_start_positions_for_source_mapping() {
    let tokens = lex("new Int value = 90deg\n    output(value)\n").expect("lex should pass");
    let new_token = &tokens[0];
    assert_eq!((new_token.line, new_token.col), (1, 1));
    let angle_unit = tokens
        .iter()
        .find(|token| token.lexeme == "deg")
        .expect("angle unit token");
    assert_eq!((angle_unit.line, angle_unit.col), (1, 19));
    let output = tokens
        .iter()
        .find(|token| token.lexeme == "output")
        .expect("output token");
    assert_eq!((output.line, output.col), (2, 5));
}
