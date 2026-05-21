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
    let src = "new x = 1 + 2 * 3\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert_eq!(program.statements.len(), 1);

    let Statement::VarDecl { value, .. } = &program.statements[0] else {
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
    let src = "new x = a and b or c\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let Statement::VarDecl { value, .. } = &program.statements[0] else {
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

#[test]
fn parses_danger_function_declaration() {
    let src = r#"
danger fn parse_value(input) {
    new x = input
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::FunctionDef { name, is_danger, .. } => {
            assert_eq!(name, "parse_value");
            assert!(*is_danger);
        }
        _ => panic!("expected FunctionDef"),
    }
}

#[test]
fn parses_if_and_while_bodies() {
    let src = r#"
if x {
    new y = 1
} else {
    y = 2
}

while y {
    y = y - 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");

    assert_eq!(program.statements.len(), 2);

    match &program.statements[0] {
        Statement::IfStatement { then_block, else_block, .. } => {
            assert_eq!(then_block.statements.len(), 1);
            assert!(else_block.is_some());
            let else_block = else_block.as_ref().expect("else block");
            assert_eq!(else_block.statements.len(), 1);
        }
        _ => panic!("expected IfStatement"),
    }

    match &program.statements[1] {
        Statement::WhileLoop { body, .. } => {
            assert_eq!(body.statements.len(), 1);
        }
        _ => panic!("expected WhileLoop"),
    }
}

#[test]
fn parses_return_statement_in_function() {
    let src = r#"
fn add(a, b) {
    return a + b
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");

    let Statement::FunctionDef { body, .. } = &program.statements[0] else {
        panic!("expected FunctionDef");
    };
    assert_eq!(body.statements.len(), 1);
    assert!(matches!(body.statements[0], Statement::ReturnStatement { .. }));
}

#[test]
fn parses_typed_new_declaration() {
    let src = "new Float t = 21.5\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let Statement::VarDecl { name, declared_type, .. } = &program.statements[0] else {
        panic!("expected VarDecl");
    };
    assert_eq!(name, "t");
    assert_eq!(declared_type.as_deref(), Some("Float"));
}

#[test]
fn parses_typed_function_signature() {
    let src = r#"
fn add(Int a, Int b) Int {
    return a + b
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let Statement::FunctionDef { params, returns, .. } = &program.statements[0] else {
        panic!("expected FunctionDef");
    };
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].param_type.as_deref(), Some("Int"));
    assert_eq!(params[0].name, "a");
    assert_eq!(params[1].param_type.as_deref(), Some("Int"));
    assert_eq!(params[1].name, "b");
    assert_eq!(returns.as_deref(), Some("Int"));
}

#[test]
fn parses_danger_on_error_assignment() {
    let src = r#"
new x = 0
x = parse_value(x) on error {
    x = 0
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert_eq!(program.statements.len(), 2);
    match &program.statements[1] {
        Statement::DangerAssignOnError { target, call_name, args, on_error } => {
            assert_eq!(target, "x");
            assert_eq!(call_name, "parse_value");
            assert_eq!(args.len(), 1);
            assert_eq!(on_error.statements.len(), 1);
        }
        _ => panic!("expected DangerAssignOnError"),
    }
}
