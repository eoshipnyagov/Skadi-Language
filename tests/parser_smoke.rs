use v01::ast_nodes::{Expression, Statement};
use v01::lexer::lex;
use v01::parser::parse_program;

#[test]
fn parses_label_and_struct_top_level() {
    let src = r#"
label Status {
    Ok
    Error
}

struct Sensor {
    u8 address
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert_eq!(program.statements.len(), 2);
    match &program.statements[0] {
        Statement::LabelDecl { name, variants } => {
            assert_eq!(name, "Status");
            assert!(variants.contains(&"Ok".to_string()));
            assert!(variants.contains(&"Error".to_string()));
        }
        _ => panic!("expected LabelDecl"),
    }
    match &program.statements[1] {
        Statement::StructDecl { name } => assert_eq!(name, "Sensor"),
        _ => panic!("expected StructDecl"),
    }
}

#[test]
fn parses_on_interrupt_block() {
    let src = r#"
on interrupt timer0 {
    output("tick")
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::OnBlock { trigger } => assert_eq!(trigger, "interrupt"),
        _ => panic!("expected OnBlock"),
    }
}

#[test]
fn parses_operator_precedence_in_assignment() {
    let src = "x = 1 + 2 * 3\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert_eq!(program.statements.len(), 1);

    let Statement::Assignment { value, .. } = &program.statements[0] else {
        panic!("expected Assignment");
    };

    let Expression::BinaryOp { op, left, right } = &**value else {
        panic!("expected top-level BinaryOp");
    };
    assert_eq!(op, "+");
    assert!(matches!(**left, Expression::LiteralInt(1)));

    let Some(right) = right else {
        panic!("expected right side");
    };
    let Expression::BinaryOp { op: rop, .. } = &**right else {
        panic!("expected nested BinaryOp");
    };
    assert_eq!(rop, "*");
}

#[test]
fn parses_word_logical_operators_precedence() {
    let src = "x = a and b or c\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let Statement::Assignment { value, .. } = &program.statements[0] else {
        panic!("expected Assignment");
    };
    let Expression::BinaryOp { op, left, right } = &**value else {
        panic!("expected BinaryOp");
    };
    assert_eq!(op, "or");
    let Expression::BinaryOp { op: lop, .. } = &**left else {
        panic!("expected left nested BinaryOp");
    };
    assert_eq!(lop, "and");
    assert!(right.is_some());
}

#[test]
fn parses_for_in_loop() {
    let src = r#"
for item in items {
    x = item
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::ForLoop { initialization, condition, .. } => {
            assert!(initialization.is_some());
            assert!(condition.is_some());
        }
        _ => panic!("expected ForLoop"),
    }
}
