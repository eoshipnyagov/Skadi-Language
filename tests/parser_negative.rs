use v01::lexer::lex;
use v01::parser::parse_program;

fn parse_err(src: &str) -> String {
    let tokens = lex(src).expect("lex should succeed");
    parse_program(&tokens).expect_err("parse should fail")
}

#[test]
fn parser_rejects_for_without_in_keyword() {
    let src = r#"
for item items {
    x = item
}
"#;
    let err = parse_err(src);
    assert!(err.contains("SC-PARSE"));
}

#[test]
fn parser_rejects_iterate_without_as_keyword() {
    let src = r#"
iterate items item {
    x = item
}
"#;
    let err = parse_err(src);
    assert!(err.contains("SC-PARSE"));
}

#[test]
fn parser_rejects_when_without_block() {
    let src = "when x is 1 { new y = 1 }\n";
    let err = parse_err(src);
    assert!(err.contains("SC-PARSE"));
}

#[test]
fn parser_rejects_on_error_without_block() {
    let src = r#"
new Int x = 1
x = parse(x) on error
"#;
    let err = parse_err(src);
    assert!(err.contains("SC-PARSE-136"));
}

#[test]
fn parser_rejects_new_without_initializer() {
    let src = "new Int x\n";
    let err = parse_err(src);
    assert!(err.contains("SC-PARSE"));
}

#[test]
fn parser_rejects_iterate_without_block() {
    let src = "iterate xs as item\n";
    let err = parse_err(src);
    assert!(err.contains("SC-PARSE"));
}

#[test]
fn parser_rejects_dotted_builtin_on_error_form() {
    let src = r#"
new Text List xs = []
xs = fs.list(".") on error {
    xs = []
}
"#;
    let err = parse_err(src);
    assert!(err.contains("SC-PARSE-134"));
}

#[test]
fn parser_structural_diagnostic_snapshot() {
    let src = r#"
new Int x = 1
x = parse(x) on error
"#;
    let err = parse_err(src);
    assert!(err.starts_with("Parse error at line"));
    assert!(err.contains("col"));
    assert!(err.contains("index"));
    assert!(err.contains("[SC-PARSE-003]"));
    assert!(err.contains("[SC-PARSE-136] on error expected '{'."));
}

#[test]
fn parser_expression_diagnostic_snapshot() {
    let src = r#"
new Int i = 0
new Int x = i++
"#;
    let err = parse_err(src);
    assert!(err.starts_with("Parse error at line"));
    assert!(err.contains("col"));
    assert!(err.contains("index"));
    assert!(err.contains("[SC-PARSE-003]"));
    assert!(err.contains("[SC-PARSE-213]"));
    assert!(err.contains("unexpected trailing token in expression"));
}

#[test]
fn parser_rejects_local_for_non_declaration_statement() {
    let src = "local new Int x = 1\n";
    let err = parse_err(src);
    assert!(err.contains("SC-PARSE-162"));
}
