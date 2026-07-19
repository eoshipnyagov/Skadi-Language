use v01::codegen::transpile_program_to_c;
use v01::formatter::format_source;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn semantic_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex vector source");
    let program = parse_program(&tokens).expect("parse vector source");
    semantic_analyze(&program).expect("semantic vector source");
    program
}

fn semantic_err(source: &str) -> String {
    let tokens = lex(source).expect("lex vector source");
    let program = parse_program(&tokens).expect("parse vector source");
    semantic_analyze(&program).expect_err("expected vector semantic error")
}

#[test]
fn semantic_accepts_vector_construction_components_and_arithmetic() {
    semantic_ok(
        r#"
new Vec2 a = {x = 1, y = 2.5}
new Vec2 b = {x = 3.0, y = 4.0}
new Vec2 sum = a + b
new Vec2 delta = sum - a
new Vec2 scaled = delta * 2
new Vec2 mirrored = -scaled
new Vec2 half = mirrored / 2.0
new Float x = half.x
half.y = x
half = delta
"#,
    );
}

#[test]
fn semantic_accepts_vector_builtins_for_all_dimensions() {
    semantic_ok(
        r#"
new Vec2 a2 = {x = 3.0, y = 4.0}
new Vec2 b2 = normalize(a2)
new Float d2 = dot(a2, b2)
new Float l2 = length(a2)
new Float ls2 = length_sq(a2)
new Float gap2 = distance(a2, b2)
new Float gap_sq2 = distance_sq(a2, b2)

new Vec3 a3 = {x = 1.0, y = 0.0, z = 0.0}
new Vec3 b3 = {x = 0.0, y = 1.0, z = 0.0}
new Vec3 normal = cross(a3, b3)

new Vec4 a4 = {x = 1.0, y = 2.0, z = 3.0, w = 4.0}
new Vec4 b4 = normalize(a4)
new Float d4 = dot(a4, b4)
"#,
    );
}

#[test]
fn semantic_rejects_invalid_vector_shapes_and_operations() {
    let missing = semantic_err("new Vec3 value = {x = 1.0, y = 2.0}\n");
    assert!(
        missing.contains("Vec3 construction requires exactly fields x, y, z"),
        "{missing}"
    );

    let extra = semantic_err("new Vec2 value = {x = 1.0, y = 2.0, z = 3.0}\n");
    assert!(
        extra.contains("Vec2 construction requires exactly fields x, y"),
        "{extra}"
    );

    let component = semantic_err("new Vec2 value = {x = \"bad\", y = 2.0}\n");
    assert!(
        component.contains("Vec2 field 'x' expects Int or Float"),
        "{component}"
    );

    let dimensions = semantic_err(
        "new Vec2 a = {x = 1.0, y = 2.0}\nnew Vec3 b = {x = 1.0, y = 2.0, z = 3.0}\nnew Float value = dot(a, b)\n",
    );
    assert!(
        dimensions.contains("expects two vectors of the same dimension"),
        "{dimensions}"
    );

    let cross = semantic_err(
        "new Vec2 a = {x = 1.0, y = 2.0}\nnew Vec2 b = {x = 3.0, y = 4.0}\nnew Vec3 value = cross(a, b)\n",
    );
    assert!(
        cross.contains("builtin 'cross' expects (Vec3, Vec3)"),
        "{cross}"
    );
}

#[test]
fn vector_values_cross_supported_value_boundaries() {
    let program = semantic_ok(
        r#"
fn make_axis() returns Vec3 {
    return {x = 1.0, y = 0.0, z = 0.0}
}

new Vec3 axis = make_axis()
new Vec3 List axes = [axis, {x = 0.0, y = 1.0, z = 0.0}]
Task(Vec3) axis_task = run make_axis()
new Vec3 task_axis = wait axis_task
Channel(Vec3) axis_channel = channel(1)
axis_channel.send(task_axis)
new Vec3 received = axis_channel.receive()
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkadiList_vec3 axes"), "{c}");
    assert!(c.contains("sk_channel_send_Vec3"), "{c}");
    assert!(c.contains("sk_channel_receive_Vec3"), "{c}");
    assert!(c.contains("Vec3 result;"), "{c}");
}

#[test]
fn formatter_keeps_canonical_vector_literal_shape() {
    let formatted = format_source("new Vec3 point={z=3,y=2,x=1}\n").expect("format vector source");
    assert_eq!(formatted, "new Vec3 point = {x = 1, y = 2, z = 3}\n");
}

#[test]
fn codegen_uses_vector_value_helpers() {
    let program = semantic_ok(
        r#"
new Vec3 a = {x = 1.0, y = 0.0, z = 0.0}
new Vec3 b = {x = 0.0, y = 1.0, z = 0.0}
new Vec3 normal = normalize(cross(a, b))
new Vec3 mixed = (a + b) * 2.0
new Float similarity = dot(normal, mixed)
new Float separation = distance(a, b)
output(normal.z)
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(
        c.contains("typedef struct { double x; double y; double z; } Vec3;"),
        "{c}"
    );
    assert!(c.contains("sk_vec3_normalize(sk_vec3_cross(a, b))"), "{c}");
    assert!(c.contains("sk_vec3_scale(sk_vec3_add(a, b), 2"), "{c}");
    assert!(c.contains("sk_vec3_dot(normal, mixed)"), "{c}");
    assert!(c.contains("sk_vec3_distance(a, b)"), "{c}");
    assert!(c.contains("sk_output_float(normal.z)"), "{c}");
}
