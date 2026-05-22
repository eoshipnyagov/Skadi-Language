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
        Statement::LabelDecl { name, variants, .. } => {
            assert_eq!(name, "Status");
            assert!(variants.contains(&"Ok".to_string()));
            assert!(variants.contains(&"Error".to_string()));
        }
        _ => panic!("expected LabelDecl"),
    }
    match &program.statements[1] {
        Statement::StructDecl { name, .. } => assert_eq!(name, "Sensor"),
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
        Statement::OnBlock { trigger, .. } => assert_eq!(trigger, "interrupt"),
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
fn parses_iterate_as_loop_alias() {
    let src = r#"
iterate items as item {
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
fn parses_typed_list_new_declaration_with_literal() {
    let src = "new i32 List xs = [1, 2, 3]\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let Statement::VarDecl { name, declared_type, value, .. } = &program.statements[0] else {
        panic!("expected VarDecl");
    };
    assert_eq!(name, "xs");
    assert_eq!(declared_type.as_deref(), Some("i32 List"));
    let Expression::ListLiteral(items) = &**value else {
        panic!("expected ListLiteral");
    };
    assert_eq!(items.len(), 3);
}

#[test]
fn parses_index_expression_in_declaration() {
    let src = r#"
new i32 List xs = [1, 2, 3]
new i32 x = xs[1]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let Statement::VarDecl { value, .. } = &program.statements[1] else {
        panic!("expected VarDecl");
    };
    assert!(matches!(**value, Expression::Index { .. }));
}

#[test]
fn parses_text_literal_declaration() {
    let src = r#"
new Text t = "hello"
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let Statement::VarDecl { declared_type, value, .. } = &program.statements[0] else {
        panic!("expected VarDecl");
    };
    assert_eq!(declared_type.as_deref(), Some("Text"));
    assert!(matches!(**value, Expression::LiteralString(_)));
}

#[test]
fn parses_list_push_statement() {
    let src = r#"
new i32 List xs = [1, 2]
xs.push(3)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert!(matches!(program.statements[1], Statement::ListPush { .. }));
}

#[test]
fn parses_list_pop_on_error_statement() {
    let src = r#"
new i32 List xs = [1, 2]
new i32 x = 0
x = xs.pop() on error {
    x = 0
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert!(matches!(program.statements[2], Statement::ListPopOnError { .. }));
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
        Statement::DangerAssignOnError { target, call_name, args, on_error, .. } => {
            assert_eq!(target, "x");
            assert_eq!(call_name, "parse_value");
            assert_eq!(args.len(), 1);
            assert_eq!(on_error.statements.len(), 1);
        }
        _ => panic!("expected DangerAssignOnError"),
    }
}

#[test]
fn parses_return_error_statement() {
    let src = r#"
danger fn parse_value(Int x) Int {
    return error ZeroDivision
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let Statement::FunctionDef { body, .. } = &program.statements[0] else {
        panic!("expected FunctionDef");
    };
    assert!(matches!(body.statements[0], Statement::ReturnError { .. }));
}

#[test]
fn parses_call_expression_in_declaration() {
    let src = r#"
fn add(Int a, Int b) Int {
    return a + b
}
new Int x = 1
new Int y = add(x, 2)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert_eq!(program.statements.len(), 3);
    let Statement::VarDecl { value, .. } = &program.statements[2] else {
        panic!("expected VarDecl");
    };
    assert!(matches!(**value, Expression::Call { .. }));
}

#[test]
fn parses_when_statement_cases_and_else() {
    let src = r#"
when x {
    is 1 {
        new y = 10
    }
    is 2, 3 {
        new y = 20
    }
    else {
        new y = 0
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    assert_eq!(program.statements.len(), 1);
    let Statement::WhenBlock { cases, else_block, .. } = &program.statements[0] else {
        panic!("expected WhenBlock");
    };
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[0].0.len(), 1);
    assert_eq!(cases[1].0.len(), 2);
    assert!(else_block.is_some());
}

#[test]
fn parse_error_reports_line_and_col() {
    let src = "fn broken(a, b\n";
    let tokens = lex(src).expect("lex should succeed");
    let err = parse_program(&tokens).expect_err("parse should fail");
    assert!(err.contains("SC-PARSE-003"));
    assert!(err.contains("SC-PARSE-106"));
    assert!(err.contains("line"));
    assert!(err.contains("col"));
}

#[test]
fn parses_dotted_builtin_call_expression() {
    let src = r#"
new Text root = "."
new Text List entries = fs.list(root)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let Statement::VarDecl { value, .. } = &program.statements[1] else {
        panic!("expected VarDecl");
    };
    let Expression::Call { name, args } = &**value else {
        panic!("expected Call");
    };
    assert_eq!(name, "fs.list");
    assert_eq!(args.len(), 1);
}
