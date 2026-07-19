use v01::ast_nodes::{Expression, Statement};
use v01::codegen::transpile_program_to_c;
use v01::formatter::format_source;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

fn parse_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex transition source");
    parse_program(&tokens).expect("parse transition source")
}

#[test]
fn compound_assignment_desugars_without_losing_the_left_operand() {
    let program = parse_ok("new Int count = 1\ncount += 2\n");
    let Statement::Assignment { target, value, .. } = &program.statements[1] else {
        panic!("expected assignment");
    };
    assert_eq!(target, "count");
    assert!(matches!(
        value.as_ref(),
        Expression::BinaryOp { op, left, right: Some(right) }
            if op == "+"
                && matches!(left.as_ref(), Expression::VariableReference(name) if name == "count")
                && matches!(right.as_ref(), Expression::LiteralInt(2))
    ));
}

#[test]
fn compound_assignment_uses_existing_type_rules_and_codegen() {
    let program = parse_ok(
        r#"
new Int count = 8
count += 2
count -= 1
count *= 3
count /= 9

new Vec2 velocity = {x = 1.0, y = 2.0}
velocity *= 2.0
velocity.x += 1.0
"#,
    );
    semantic_analyze(&program).expect("compound assignment semantics");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("count = (count + 2);"), "{c}");
    assert!(c.contains("count = (count - 1);"), "{c}");
    assert!(c.contains("count = (count * 3);"), "{c}");
    assert!(c.contains("count = (count / 9);"), "{c}");
    assert!(c.contains("velocity = sk_vec2_scale(velocity, 2"), "{c}");
    assert!(c.contains("velocity.x = (velocity.x + 1"), "{c}");
}

#[test]
fn formatter_canonicalizes_compound_assignment_and_legacy_returns() {
    let formatted = format_source(
        r#"
fn add(Int a,Int b) Int{
new Int result=a
result+=b
return result
}
"#,
    )
    .expect("format transition source");
    assert_eq!(
        formatted,
        "fn add(Int a, Int b) returns Int {\n    new Int result = a\n    result = result + b\n    return result\n}\n"
    );
}

#[test]
fn compound_assignment_is_not_accepted_as_a_declaration_initializer() {
    let tokens = lex("new Int count += 1\n").expect("lex invalid declaration");
    let err = parse_program(&tokens).expect_err("compound declaration must fail");
    assert!(err.contains("SC-PARSE-140"), "{err}");
    assert!(err.contains("expected '='"), "{err}");
}

#[test]
fn legacy_user_type_return_is_compatible_but_not_canonical() {
    let program = parse_ok(
        r#"
struct Point {
    Float x
}
fn origin() Point {
    return {x = 0.0}
}
"#,
    );
    semantic_analyze(&program).expect("legacy return compatibility");
    let warnings = semantic_style_warnings(&program);
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("prefer explicit 'returns <type>'")),
        "{warnings:?}"
    );
}

#[test]
fn on_interrupt_is_parseable_but_explicitly_not_compilable() {
    let program = parse_ok("on interrupt timer0 {\n    pass\n}\n");
    let err = semantic_analyze(&program).expect_err("on interrupt must not reach codegen");
    assert!(err.contains("SC-SEM-040"), "{err}");
    assert!(err.contains("future platform runtime"), "{err}");
}

#[test]
fn legacy_c_style_for_is_parseable_but_explicitly_not_compilable() {
    let program = parse_ok("for (i = 0; i < 10; i++) {\n    pass\n}\n");
    let err = semantic_analyze(&program).expect_err("legacy C-style for must not reach codegen");
    assert!(err.contains("SC-SEM-040"), "{err}");
    assert!(
        err.contains("parse/format compatibility syntax only"),
        "{err}"
    );
    assert!(err.contains("iterate collection as item"), "{err}");
}

#[test]
fn untyped_scalar_declarations_receive_their_inferred_c_type() {
    let program = parse_ok(
        "new count = 1\nnew ratio = 1.5\nnew enabled = true\nnew letter = 'a'\nnew title = \"Skadi\"\n",
    );
    semantic_analyze(&program).expect("scalar inference must stay supported");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t count = 1;"), "{c}");
    assert!(c.contains("double ratio = 1.5"), "{c}");
    assert!(c.contains("bool enabled = true;"), "{c}");
    assert!(c.contains("char letter = 'a';"), "{c}");
    assert!(c.contains("const char* title = \"Skadi\";"), "{c}");
}

#[test]
fn untyped_composite_declarations_are_rejected_before_codegen() {
    for source in ["new values = [1, 2]\n", "new point = {x = 1.0, y = 2.0}\n"] {
        let program = parse_ok(source);
        let err = semantic_analyze(&program).expect_err("composite type must be explicit");
        assert!(err.contains("SC-SEM-020"), "{err}");
        assert!(
            err.contains("explicit type required for composite declaration"),
            "{err}"
        );
    }
}

#[test]
fn char_literals_are_typed_formatted_and_lowered() {
    let program = parse_ok(
        r#"
new Char letter = 'a'
new Char newline = '\n'
new Char quote = '\''
new Char List letters = [letter, newline, quote]
output(letter)
"#,
    );
    assert!(matches!(
        &program.statements[0],
        Statement::VarDecl { value, .. }
            if matches!(value.as_ref(), Expression::LiteralChar('a'))
    ));
    semantic_analyze(&program).expect("Char literals must pass semantics");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("char letter = 'a';"), "{c}");
    assert!(c.contains("char newline = '\\n';"), "{c}");
    assert!(c.contains("SkadiList_char letters"), "{c}");
    assert!(c.contains("sk_output_char(letter)"), "{c}");

    let formatted = format_source("new Char value='\\t'\n").expect("format Char literal");
    assert_eq!(formatted, "new Char value = '\\t'\n");
}

#[test]
fn char_literals_reject_empty_multi_character_and_non_ascii_values() {
    for (source, expected) in [
        ("new Char value = ''\n", "cannot be empty"),
        ("new Char value = 'ab'\n", "exactly one character"),
        ("new Char value = 'я'\n", "must be ASCII"),
        ("new Char value = '\\x'\n", "supported escape"),
    ] {
        let tokens = lex(source).expect("lex invalid Char literal");
        let err = parse_program(&tokens).expect_err("invalid Char literal must fail");
        assert!(err.contains("SC-PARSE-221"), "{err}");
        assert!(err.contains(expected), "{err}");
    }
}
