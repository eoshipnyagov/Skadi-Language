use v01::lexer::lex;

#[test]
fn lex_error_reports_line_and_col_in_unified_style() {
    let src = "new x = 1\n@";
    let err = lex(src).expect_err("lex should fail on unexpected character");
    let rendered = err.to_string();
    assert!(rendered.contains("Lex error at line"));
    assert!(rendered.contains("col"));
    assert!(rendered.contains("Unexpected character"));
}
