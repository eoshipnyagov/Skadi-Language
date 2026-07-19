use v01::ast_nodes::{Expression, Statement};
use v01::codegen::transpile_program_to_c;
use v01::formatter::format_source;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn parse_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex ByteSize source");
    parse_program(&tokens).expect("parse ByteSize source")
}

fn semantic_ok(source: &str) -> v01::ast_nodes::Program {
    let program = parse_ok(source);
    semantic_analyze(&program).expect("semantic ByteSize source");
    program
}

fn semantic_err(source: &str) -> String {
    let program = parse_ok(source);
    semantic_analyze(&program).expect_err("expected ByteSize semantic error")
}

#[test]
fn parser_accepts_byte_size_literals_and_preserves_units() {
    let program = parse_ok(
        r#"
new ByteSize bytes = 7b
new ByteSize kibibytes = 2kb
new ByteSize mebibytes = 3mb
new ByteSize gibibytes = 4gb
"#,
    );

    let expected = [
        (7, "b", 7),
        (2, "kb", 2 * 1024),
        (3, "mb", 3 * 1024 * 1024),
        (4, "gb", 4 * 1024 * 1024 * 1024),
    ];
    for (statement, (magnitude, unit, bytes)) in program.statements.iter().zip(expected) {
        assert!(matches!(
            statement,
            Statement::VarDecl { value, .. }
                if matches!(
                    value.as_ref(),
                    Expression::LiteralByteSize {
                        magnitude: actual_magnitude,
                        unit: actual_unit,
                        bytes: actual_bytes,
                    } if *actual_magnitude == magnitude
                        && actual_unit == unit
                        && *actual_bytes == bytes
                )
        ));
    }
}

#[test]
fn parser_rejects_fractional_spaced_and_overflowing_byte_size_literals() {
    let fractional = lex("new ByteSize value = 1.5kb\n").expect("lex fractional ByteSize");
    let err = parse_program(&fractional).expect_err("fractional ByteSize must fail");
    assert!(err.contains("SC-PARSE-219"), "{err}");
    assert!(err.contains("integer magnitude"), "{err}");

    let spaced = lex("new ByteSize value = 1 kb\n").expect("lex spaced ByteSize");
    parse_program(&spaced).expect_err("spaced general ByteSize literal must fail");

    let overflow =
        lex("new ByteSize value = 9223372036854775807gb\n").expect("lex ByteSize overflow");
    let err = parse_program(&overflow).expect_err("overflowing ByteSize must fail");
    assert!(err.contains("SC-PARSE-219"), "{err}");
    assert!(err.contains("exceeds i64 bytes"), "{err}");
}

#[test]
fn semantic_accepts_nominal_arithmetic_and_memory_expression() {
    semantic_ok(
        r#"
new ByteSize base = 4kb
new ByteSize overhead = 512b
new ByteSize capacity = base + overhead
new ByteSize remaining = capacity - 1kb
new Bool enough = remaining >= 3kb
Memory scratch_memory = memory(capacity)
scratch_memory.clear()
"#,
    );
}

#[test]
fn semantic_rejects_numeric_mixing_and_non_byte_memory_capacity() {
    let from_int = semantic_err("new ByteSize value = 1\n");
    assert!(
        from_int.contains("cannot assign Int to ByteSize"),
        "{from_int}"
    );

    let mixed = semantic_err("new ByteSize value = 1kb + 1\n");
    assert!(
        mixed.contains("operator '+' is not defined for ByteSize and Int"),
        "{mixed}"
    );

    let invalid_memory = semantic_err("Memory arena = memory(4096)\n");
    assert!(
        invalid_memory.contains("memory(size) expects ByteSize, got Int"),
        "{invalid_memory}"
    );
}

#[test]
fn formatter_canonicalizes_byte_size_literals_and_legacy_memory_spacing() {
    let formatted = format_source(
        r#"
new ByteSize capacity=1kb+512b
Memory scratch_memory=memory(8 mb)
"#,
    )
    .expect("format ByteSize source");
    assert_eq!(
        formatted,
        "new ByteSize capacity = 1kb + 512b\n\nMemory scratch_memory = memory(8mb)\n"
    );
}

#[test]
fn codegen_uses_checked_dynamic_memory_capacity() {
    let program = semantic_ok(
        r#"
new ByteSize base = 2kb
new ByteSize capacity = base + 512b
Memory scratch_memory = memory(capacity) on error {
    output("capacity rejected")
}
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t base = 2048;"), "{c}");
    assert!(c.contains("int64_t capacity = (base + 512);"), "{c}");
    assert!(
        c.contains("int64_t scratch_memory_capacity = capacity;"),
        "{c}"
    );
    assert!(c.contains("scratch_memory_capacity <= 0 || !sk_mem_region_init"));
    assert!(c.contains("(size_t)scratch_memory_capacity"));
}

#[test]
fn byte_size_values_cross_supported_container_and_task_boundaries() {
    let program = semantic_ok(
        r#"
fn calculate() returns ByteSize {
    return 2kb
}

Task(ByteSize) calculation_task = run calculate()
new ByteSize result = wait calculation_task
new ByteSize List sizes = [1kb, result]
Channel(ByteSize) capacities = channel(1)
capacities.send(result)
new ByteSize received = capacities.receive()
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkadiList_bytesize sizes"), "{c}");
    assert!(c.contains("sk_channel_send_ByteSize"), "{c}");
    assert!(c.contains("sk_channel_receive_ByteSize"), "{c}");
    assert!(c.contains("int64_t result;"), "{c}");
}
