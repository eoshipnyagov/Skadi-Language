use v01::ast_nodes::{Program, Statement};
use v01::codegen::transpile_program_to_c;
use v01::formatter::format_source;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn parse_ok(source: &str) -> Program {
    let tokens = lex(source).expect("lex C ABI source");
    parse_program(&tokens).expect("parse C ABI source")
}

fn semantic_ok(source: &str) -> Program {
    let program = parse_ok(source);
    semantic_analyze(&program).expect("semantic C ABI source");
    program
}

fn semantic_err(source: &str) -> String {
    let program = parse_ok(source);
    semantic_analyze(&program).expect_err("expected C ABI semantic error")
}

#[test]
fn external_scalar_declarations_are_typed_and_lowered_to_prototypes() {
    let program = semantic_ok(
        r#"
external fn c_add(i32 left, i32 right) returns i32
external fn c_ping()

new i32 answer = c_add(20, 22)
c_ping()
"#,
    );
    let c = transpile_program_to_c(&program);
    let add = "int32_t c_add(int32_t left, int32_t right);";
    let ping = "void c_ping(void);";
    assert!(c.contains(add), "{c}");
    assert!(c.contains(ping), "{c}");
    assert!(c.find(add) < c.find("int main(void)"), "{c}");
    assert!(!c.contains("c_add(int32_t left, int32_t right) {"), "{c}");
}

#[test]
fn external_struct_values_are_typed_and_lowered_with_c_field_order() {
    let program = semantic_ok(
        r#"
external struct SensorReading {
    i32 value
    f32 confidence
    Bool valid
}

external fn sensor_describe(i32 value) returns SensorReading
external danger fn sensor_read(i32 channel) returns SensorReading

new SensorReading reading = sensor_describe(7)
reading = sensor_read(1) on error {
    output("read failed")
}
"#,
    );
    let Statement::StructDecl {
        is_external: true,
        fields,
        ..
    } = &program.statements[0]
    else {
        panic!("expected external struct");
    };
    assert_eq!(fields.len(), 3);

    let c = transpile_program_to_c(&program);
    assert!(
        c.contains(
            "typedef struct {\n    int32_t value;\n    float confidence;\n    bool valid;\n} SensorReading;"
        ),
        "{c}"
    );
    assert!(
        c.contains("SensorReading sensor_describe(int32_t value);"),
        "{c}"
    );
    assert!(
        c.contains("int sensor_read(int32_t channel, SensorReading *out);"),
        "{c}"
    );
}

#[test]
fn formatter_preserves_bodyless_external_declarations() {
    let formatted =
        format_source(
            "external fn c_add(i32 left,i32 right) returns i32\nexternal fn c_sum(view Buffer(u8) data) returns u32\nexternal danger fn c_zero(edit Buffer(u8) data)\nexternal fn c_ping()\n",
        )
            .expect("format external declarations");
    assert_eq!(
        formatted,
        "external fn c_add(i32 left, i32 right) returns i32\n\nexternal fn c_sum(view Buffer(u8) data) returns u32\n\nexternal danger fn c_zero(edit Buffer(u8) data)\n\nexternal fn c_ping()\n"
    );
}

#[test]
fn formatter_preserves_external_struct_layout() {
    let formatted =
        format_source("external struct Reading {\ni32 value\nf32 confidence\nBool valid\n}\n")
            .expect("format external struct");
    assert_eq!(
        formatted,
        "external struct Reading {\n    i32 value\n    f32 confidence\n    Bool valid\n}\n"
    );
}

#[test]
fn external_danger_uses_the_existing_status_and_out_abi() {
    let program = semantic_ok(
        r#"
external danger fn c_read(i32 channel) returns i32

new i32 value = 0
value = c_read(2) on error {
    value = -1
}
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(
        c.contains("int c_read(int32_t channel, int32_t *out);"),
        "{c}"
    );
    assert!(c.contains("if (c_read(2, &value) != 0)"), "{c}");

    let unhandled = semantic_err(
        "external danger fn c_read(i32 channel) returns i32\nnew i32 value = c_read(2)\n",
    );
    assert!(unhandled.contains("SC-SEM-040"), "{unhandled}");
    assert!(unhandled.contains("requires 'on error'"), "{unhandled}");
}

#[test]
fn external_scalar_parameters_reject_borrows_and_bodies() {
    let borrowed_error = semantic_err("external fn inspect(view i32 value)\n");
    assert!(borrowed_error.contains("SC-SEM-040"), "{borrowed_error}");
    assert!(
        borrowed_error.contains("must be passed by value"),
        "{borrowed_error}"
    );

    let body = lex("external fn c_ping() {\n    pass\n}\n").expect("lex external body");
    let body_error = parse_program(&body).expect_err("external body must fail");
    assert!(body_error.contains("SC-PARSE-229"), "{body_error}");
    assert!(
        body_error.contains("must end after its return type"),
        "{body_error}"
    );
}

#[test]
fn external_buffers_lower_typed_lists_to_pointer_and_length_pairs() {
    let program = semantic_ok(
        r#"
external fn checksum(view Buffer(u8) data) returns u32
external danger fn zero(edit Buffer(u8) data)

new u8 List payload = [1, 2, 3]
new u32 sum = checksum(view payload)
zero(edit payload) on error {
    output("zero failed")
}
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(
        c.contains("uint32_t checksum(const uint8_t *data, size_t data_length);"),
        "{c}"
    );
    assert!(
        c.contains("int zero(uint8_t *data, size_t data_length);"),
        "{c}"
    );
    assert!(c.contains("checksum(payload.data, payload.len)"), "{c}");
    assert!(
        c.contains("if (zero(payload.data, payload.len) != 0)"),
        "{c}"
    );
}

#[test]
fn external_buffers_are_call_scoped_and_element_typed() {
    for source in [
        "external fn checksum(Buffer(u8) data) returns u32\n",
        "external fn checksum(move Buffer(u8) data) returns u32\n",
        "external fn checksum(view Buffer(Text) data) returns u32\n",
        "external fn checksum(view Buffer(u8) data) returns Buffer(u8)\n",
        "fn inspect(view Buffer(u8) data) {\n    pass\n}\n",
    ] {
        let error = semantic_err(source);
        assert!(error.contains("SC-SEM-040"), "{error}");
    }

    let wrong_list = semantic_err(
        "external fn checksum(view Buffer(u8) data) returns u32\nnew u16 List data = [1]\nnew u32 value = checksum(view data)\n",
    );
    assert!(wrong_list.contains("element mismatch"), "{wrong_list}");

    let stored = lex("new Buffer(u8) data = [1, 2]\n").expect("lex stored Buffer");
    let stored_error = parse_program(&stored).expect_err("Buffer must not be storable");
    assert!(stored_error.contains("Parse error"), "{stored_error}");
}

#[test]
fn abbreviated_extern_spelling_is_not_an_external_declaration() {
    let tokens = lex("extern fn c_ping()\n").expect("lex obsolete spelling");
    let error = parse_program(&tokens).expect_err("extern alias must not be accepted");
    assert!(error.contains("Parse error"), "{error}");
}

#[test]
fn external_contract_accepts_only_fixed_scalar_abi_types() {
    for source in [
        "external fn platform_value(Int value) returns i32\n",
        "external fn text_value(Text value) returns i32\n",
        "external fn custom_value(Player value) returns i32\n",
        "external fn bad_return(i32 value) returns Int\n",
    ] {
        let error = semantic_err(source);
        assert!(error.contains("SC-SEM-040"), "{error}");
        assert!(error.contains("unsupported"), "{error}");
    }
}

#[test]
fn external_struct_rejects_ambiguous_or_non_c_layout_surface() {
    for source in [
        "external struct Empty {\n}\n",
        "external struct TextField {\n    Text value\n}\n",
        "external struct HiddenField {\n    hide i32 value\n}\n",
        "external struct Duplicate {\n    i32 value\n    i32 value\n}\n",
        "struct Regular {\n    i32 value\n}\nexternal fn relay(Regular value) returns i32\n",
    ] {
        let error = semantic_err(source);
        assert!(error.contains("SC-SEM-"), "{error}");
    }

    let method_tokens = lex(
        "external struct WithMethod {\n    i32 value\n    fn get() {\n        pass\n    }\n}\n",
    )
    .expect("lex external method");
    let method_error = parse_program(&method_tokens).expect_err("external method must fail");
    assert!(method_error.contains("SC-PARSE-231"), "{method_error}");
    assert!(
        method_error.contains("cannot declare methods"),
        "{method_error}"
    );
}

#[test]
fn void_external_cannot_be_used_as_a_value() {
    let error = semantic_err("external fn c_ping()\nnew i32 value = c_ping()\n");
    assert!(error.contains("SC-SEM-020"), "{error}");
    assert!(error.contains("cannot assign"), "{error}");
}

#[test]
fn external_function_cannot_be_started_as_a_skadi_task() {
    let error = semantic_err(
        r#"
external fn c_add(i32 left, i32 right) returns i32
Task(i32) worker = run c_add(20, 22)
"#,
    );
    assert!(error.contains("SC-SEM-070"), "{error}");
    assert!(error.contains("external fn 'c_add'"), "{error}");
}
