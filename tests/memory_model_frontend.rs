use v01::ast_nodes::Statement;
use v01::codegen::ensure_codegen_supported;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn parse_ok(src: &str) -> v01::ast_nodes::Program {
    let tokens = lex(src).expect("lex should succeed");
    parse_program(&tokens).expect("parse should succeed")
}

fn parse_err(src: &str) -> String {
    let tokens = lex(src).expect("lex should succeed");
    parse_program(&tokens).expect_err("parse should fail")
}

#[test]
fn parser_accepts_memory_declaration_place_in_and_clear() {
    let src = r#"
Memory arena = memory(8mb) on error {
    output("oom")
}

place in arena on error {
    output("overflow")
} {
    new Text msg = "hello"
}

arena.clear()
"#;
    let program = parse_ok(src);
    assert_eq!(program.statements.len(), 3);

    match &program.statements[0] {
        Statement::MemoryDecl {
            name,
            size_spec,
            on_error,
            ..
        } => {
            assert_eq!(name, "arena");
            assert_eq!(size_spec.trim(), "8 mb");
            assert!(on_error.is_some());
        }
        _ => panic!("expected MemoryDecl"),
    }

    match &program.statements[1] {
        Statement::PlaceIn {
            memory_name,
            on_error,
            body,
            ..
        } => {
            assert_eq!(memory_name, "arena");
            assert!(on_error.is_some());
            assert_eq!(body.statements.len(), 1);
        }
        _ => panic!("expected PlaceIn"),
    }

    assert!(matches!(
        program.statements[2],
        Statement::MemoryClear { .. }
    ));
}

#[test]
fn parser_rejects_empty_memory_size() {
    let err = parse_err("Memory arena = memory()\n");
    assert!(err.contains("SC-PARSE-166"));
}

#[test]
fn parser_rejects_place_without_body() {
    let err = parse_err("place in arena\n");
    assert!(err.contains("SC-PARSE-173"));
}

#[test]
fn semantic_allows_return_from_external_memory_place_block() {
    let src = r#"
fn build(Memory arena) Text {
    place in arena {
        new Text msg = "hello"
        return msg
    }
}
"#;
    let program = parse_ok(src);
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_allows_local_memory_when_values_do_not_escape() {
    let src = r#"
fn warmup() Int {
    Memory scratch = memory(4mb)
    place in scratch {
        new Text msg = "hello"
        output(msg)
    }
    scratch.clear()
    return 0
}
"#;
    let program = parse_ok(src);
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_return_from_local_memory() {
    let src = r#"
fn build() Text {
    Memory scratch = memory(4mb)
    place in scratch {
        new Text msg = "hello"
        return msg
    }
}
"#;
    let program = parse_ok(src);
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-061"));
    assert!(err.contains("local Memory"));
}

#[test]
fn semantic_rejects_storing_local_region_value_into_longer_lived_owner() {
    let src = r#"
fn build() Int {
    Memory scratch = memory(4mb)
    new Text result = ""
    place in scratch {
        new Text tmp = "hello"
        result = tmp
    }
    return 0
}
"#;
    let program = parse_ok(src);
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-060"));
    assert!(err.contains("cannot be stored into a longer-lived owner"));
}

#[test]
fn semantic_rejects_obvious_use_after_clear() {
    let src = r#"
fn render(Memory arena) Int {
    place in arena {
        new Text frame = "hello"
        arena.clear()
        output(frame)
    }
    return 0
}
"#;
    let program = parse_ok(src);
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-061"));
    assert!(err.contains("use-after-clear"));
}

#[test]
fn backend_gate_rejects_memory_model_after_semantic_success() {
    let src = r#"
fn warmup(Memory arena) Int {
    place in arena {
        new Text msg = "hello"
        output(msg)
    }
    return 0
}
"#;
    let program = parse_ok(src);
    semantic_analyze(&program).expect("semantic analysis should pass");
    let err = ensure_codegen_supported(&program).expect_err("backend gate should fail");
    assert!(err.contains("SC-CG-201"));
    assert!(err.contains("memory model frontend is implemented"));
}
