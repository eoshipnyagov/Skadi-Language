use v01::ast_nodes::Statement;
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
