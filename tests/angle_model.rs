use v01::ast_nodes::{Expression, Statement};
use v01::codegen::transpile_program_to_c;
use v01::formatter::format_source;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn parse_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex Angle source");
    parse_program(&tokens).expect("parse Angle source")
}

fn semantic_ok(source: &str) -> v01::ast_nodes::Program {
    let program = parse_ok(source);
    semantic_analyze(&program).expect("semantic Angle source");
    program
}

fn semantic_err(source: &str) -> String {
    let program = parse_ok(source);
    semantic_analyze(&program).expect_err("expected Angle semantic error")
}

#[test]
fn parser_accepts_integer_and_fractional_angle_literals() {
    let program = parse_ok(
        r#"
new Angle quarter_turn = 90deg
new Angle offset = 0.25rad
"#,
    );

    let Statement::VarDecl { value, .. } = &program.statements[0] else {
        panic!("expected Angle declaration");
    };
    let Expression::LiteralAngle {
        radians,
        magnitude,
        unit,
    } = value.as_ref()
    else {
        panic!("expected Angle literal");
    };
    assert!((*radians - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert_eq!(magnitude, "90");
    assert_eq!(unit, "deg");

    assert!(matches!(
        &program.statements[1],
        Statement::VarDecl { value, .. }
            if matches!(
                value.as_ref(),
                Expression::LiteralAngle { radians, magnitude, unit }
                    if (*radians - 0.25).abs() < 1e-6
                        && magnitude == "0.25"
                        && unit == "rad"
            )
    ));
}

#[test]
fn parser_rejects_spaced_and_non_finite_angle_literals() {
    let spaced = lex("new Angle value = 90 deg\n").expect("lex spaced Angle");
    parse_program(&spaced).expect_err("spaced Angle literal must fail");

    let huge = format!("new Angle value = {}deg\n", "9".repeat(400));
    let tokens = lex(&huge).expect("lex non-finite Angle");
    let err = parse_program(&tokens).expect_err("non-finite Angle literal must fail");
    assert!(err.contains("SC-PARSE-220"), "{err}");
    assert!(err.contains("finite f32"), "{err}");
}

#[test]
fn semantic_accepts_angle_arithmetic_and_math_integration() {
    semantic_ok(
        r#"
new Angle heading = 45deg
new Angle offset = 0.25rad
new Angle combined = heading + offset
new Angle doubled = combined * 2
new Angle mirrored = -doubled
new Angle half = doubled / 2.0
new Float ratio = doubled / heading
new Bool ordered = mirrored < half
new Float dx = cos(combined)
new Float dy = sin(combined)
new Float slope = tan(combined)
new Angle from_sine = asin(0.5)
new Angle from_cosine = acos(0.5)
new Angle from_tangent = atan(1.0)
new Angle measured = atan2(dy, dx)
new Angle normalized = normalize_angle(measured + 360deg)
new Float measured_degrees = rad_to_deg(measured)
new Float measured_radians = as_radians(measured)
new Angle converted = deg_to_rad(measured_degrees)
new Bool finite = is_finite(measured)
"#,
    );
}

#[test]
fn semantic_rejects_implicit_numeric_mixing_and_wrong_conversions() {
    let from_float = semantic_err("new Angle value = 1.0\n");
    assert!(
        from_float.contains("cannot assign Float to Angle"),
        "{from_float}"
    );

    let mixed = semantic_err("new Angle value = 1rad + 1.0\n");
    assert!(
        mixed.contains("operator '+' is not defined for Angle and Float"),
        "{mixed}"
    );

    let wrong_to_degrees = semantic_err("new Float value = rad_to_deg(1.0)\n");
    assert!(
        wrong_to_degrees.contains("builtin 'rad_to_deg' expects Angle, got Float"),
        "{wrong_to_degrees}"
    );

    let wrong_from_degrees = semantic_err("new Angle value = deg_to_rad(1deg)\n");
    assert!(
        wrong_from_degrees.contains("builtin 'deg_to_rad' expects numeric degrees, got Angle"),
        "{wrong_from_degrees}"
    );

    let raw_radians = semantic_err("new Float value = sin(1.0)\n");
    assert!(
        raw_radians.contains("builtin 'sin' expects Angle, got Float"),
        "{raw_radians}"
    );
}

#[test]
fn formatter_keeps_joined_angle_literals() {
    let formatted = format_source(
        r#"
new Angle heading=45deg+0.25rad
"#,
    )
    .expect("format Angle source");
    assert_eq!(formatted, "new Angle heading = 45deg + 0.25rad\n");
}

#[test]
fn codegen_lowers_angles_to_float_radians() {
    let program = semantic_ok(
        r#"
new Angle heading = 90deg
new Float x = cos(heading)
new Angle measured = atan2(1.0, x)
new Float degrees = rad_to_deg(measured)
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("float heading = 1.570796371f;"), "{c}");
    assert!(c.contains("float x = cosf(heading);"), "{c}");
    assert!(c.contains("float measured = atan2f(1, x);"), "{c}");
    assert!(
        c.contains("float degrees = ((measured * 180.0f) / (float)M_PI);"),
        "{c}"
    );
}

#[test]
fn angle_values_cross_supported_container_and_task_boundaries() {
    let program = semantic_ok(
        r#"
fn calculate() returns Angle {
    return 90deg
}

Task(Angle) calculation_task = run calculate()
new Angle result = wait calculation_task
new Angle List headings = [45deg, result]
Channel(Angle) angles = channel(1)
angles.send(result)
new Angle received = angles.receive()
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkadiList_angle headings"), "{c}");
    assert!(c.contains("sk_channel_send_Angle"), "{c}");
    assert!(c.contains("sk_channel_receive_Angle"), "{c}");
    assert!(c.contains("float result;"), "{c}");
}
