use v01::ast_nodes::{Expression, Statement};
use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn parse_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex bit source");
    parse_program(&tokens).expect("parse bit source")
}

fn semantic_ok(source: &str) -> v01::ast_nodes::Program {
    let program = parse_ok(source);
    semantic_analyze(&program).expect("semantic bit source");
    program
}

fn semantic_err(source: &str) -> String {
    let program = parse_ok(source);
    semantic_analyze(&program).expect_err("expected bit semantic error")
}

#[test]
fn parser_accepts_prefixed_and_separated_integer_literals() {
    let program = parse_ok(
        r#"
new u8 binary = 0b1010_0101
new u16 octal = 0o17_00
new u32 hexadecimal = 0xCAFE_BABE
"#,
    );

    let values = program
        .statements
        .iter()
        .map(|statement| match statement {
            Statement::VarDecl { value, .. } => match value.as_ref() {
                Expression::LiteralInt(value) => *value,
                other => panic!("expected integer literal, got {other:?}"),
            },
            other => panic!("expected declaration, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(values, [0b1010_0101, 0o17_00, 0xCAFE_BABE]);
}

#[test]
fn semantic_accepts_bits_for_all_fixed_integer_types() {
    semantic_ok(
        r#"
new i8 a_source = 0
new i8 a = bit_not(a_source)
new i16 b_source = 0x7fff
new i16 b = bit_and(b_source, 0x00ff)
new i32 c_source = 1
new i32 c = bit_or(c_source, 2)
new i64 d_source = 7
new i64 d = bit_xor(d_source, 3)
new u8 e_source = 0
new u8 e = bit_set(e_source, 7)
new u16 f_source = 0xffff
new u16 f = bit_clear(f_source, 3)
new u32 g_source = 0
new u32 g = bit_toggle(g_source, 31)
new u64 h_source = 0
new u64 h = bit_write(h_source, 63, true)
new Bool set = bit_is_set(h, 63)
"#,
    );
}

#[test]
fn semantic_rejects_platform_int_mixed_types_and_bad_indices() {
    let platform = semantic_err("new Int value = 1\nnew Int result = bit_not(value)\n");
    assert!(platform.contains("SC-SEM-020"), "{platform}");
    assert!(platform.contains("fixed-width integer"), "{platform}");

    let mixed =
        semantic_err("new i8 left = 1\nnew u8 right = 2\nnew i8 result = bit_and(left, right)\n");
    assert!(
        mixed.contains("equal integer widths and signedness"),
        "{mixed}"
    );

    let index = semantic_err("new u8 source = 0\nnew u8 value = bit_set(source, 8)\n");
    assert!(index.contains("bit index 8 is outside 0..7"), "{index}");
}

#[test]
fn codegen_uses_unsigned_representation_for_signed_bit_shifts() {
    let program = semantic_ok(
        r#"
new i8 raw_value = -1
new i8 shifted = bit_shift_right(raw_value, 1)
new u8 empty = 0
new u8 flags = bit_set(empty, 3)
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_bit_shift_right_i8"), "{c}");
    assert!(c.contains("(uint8_t)value >> index"), "{c}");
    assert!(c.contains("sk_bit_set_u8(empty, 3)"), "{c}");
    assert!(c.contains("SC-RT-340"), "{c}");
}

#[test]
fn codegen_selects_bit_width_from_struct_fields() {
    let program = semantic_ok(
        r#"
struct Device {
    u16 status
}

new Device device = {status = 0}
new u16 masked = bit_and(device.status, 0xff)
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_bit_and_u16(device.status, 255)"), "{c}");
}
