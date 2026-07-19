use v01::ast_nodes::{Expression, Statement};
use v01::codegen::transpile_program_to_c;
use v01::formatter::format_source;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn parse_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex time model source");
    parse_program(&tokens).expect("parse time model source")
}

fn semantic_ok(source: &str) {
    let program = parse_ok(source);
    semantic_analyze(&program).expect("semantic time model source");
}

fn semantic_err(source: &str) -> String {
    let program = parse_ok(source);
    semantic_analyze(&program).expect_err("expected semantic error")
}

#[test]
fn parser_accepts_duration_literals_and_preserves_units() {
    let program = parse_ok(
        r#"
new Duration short_delay = 25ms
new Duration normal_delay = 2s
new Duration long_delay = 3min
"#,
    );

    let expected = [
        (25, "ms", 25_000_000),
        (2, "s", 2_000_000_000),
        (3, "min", 180_000_000_000),
    ];
    for (statement, (magnitude, unit, nanoseconds)) in program.statements.iter().zip(expected) {
        assert!(matches!(
            statement,
            Statement::VarDecl { value, .. }
                if matches!(
                    value.as_ref(),
                    Expression::LiteralDuration {
                        magnitude: actual_magnitude,
                        unit: actual_unit,
                        nanoseconds: actual_nanoseconds,
                    } if *actual_magnitude == magnitude
                        && actual_unit == unit
                        && *actual_nanoseconds == nanoseconds
                )
        ));
    }
}

#[test]
fn parser_rejects_fractional_and_overflowing_duration_literals() {
    let fractional = lex("new Duration value = 1.5s\n").expect("lex fractional duration");
    let err = parse_program(&fractional).expect_err("fractional duration must fail");
    assert!(err.contains("SC-PARSE-216"), "{err}");
    assert!(err.contains("integer magnitude"), "{err}");

    let overflow =
        lex("new Duration value = 999999999999999999min\n").expect("lex overflowing duration");
    let err = parse_program(&overflow).expect_err("overflowing duration must fail");
    assert!(err.contains("SC-PARSE-216"), "{err}");
    assert!(err.contains("exceeds i64 nanoseconds"), "{err}");
}

#[test]
fn semantic_accepts_nominal_time_operations() {
    semantic_ok(
        r#"
new Duration interval = 1s + 250ms
new Duration remaining = interval - 50ms
new Time started_at = now()
new Time deadline = started_at + remaining
new Duration window = deadline - started_at
new Bool enough = window >= 1s
sleep(1ms)
delay(1ms)
new Duration measured = elapsed(started_at)
"#,
    );
}

#[test]
fn semantic_rejects_implicit_numbers_and_invalid_time_arithmetic() {
    let duration_from_int = semantic_err("new Duration value = 1\n");
    assert!(duration_from_int.contains("cannot assign Int to Duration"));

    let time_from_duration = semantic_err("new Time value = 1s\n");
    assert!(time_from_duration.contains("cannot assign Duration to Time"));

    let sleep_int = semantic_err("sleep(1)\n");
    assert!(sleep_int.contains("expects (Duration), got (Int)"));

    let add_times = semantic_err(
        r#"
new Time first = now()
new Time second = now()
new Time invalid = first + second
"#,
    );
    assert!(add_times.contains("operator '+' is not defined for Time and Time"));
}

#[test]
fn formatter_keeps_canonical_duration_literals() {
    let formatted = format_source(
        r#"
new Duration interval=1s+250ms
new Time started=now()
sleep(5ms)
"#,
    )
    .expect("format time source");
    assert_eq!(
        formatted,
        "new Duration interval = 1s + 250ms\n\nnew Time started = now()\n\nsleep(5ms)\n"
    );
}

#[test]
fn codegen_emits_portable_monotonic_time_runtime() {
    let program = parse_ok(
        r#"
new Time started_at = now()
sleep(5ms)
new Duration measured = elapsed(started_at)
new Bool passed = measured >= 1ms
"#,
    );
    semantic_analyze(&program).expect("semantic time codegen source");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("QueryPerformanceCounter"));
    assert!(c.contains("clock_gettime(CLOCK_MONOTONIC"));
    assert!(c.contains("nanosleep(&request, &request)"));
    assert!(c.contains("int64_t started_at = sk_time_now();"));
    assert!(c.contains("sk_time_sleep(5000000);"));
    assert!(c.contains("int64_t measured = sk_time_elapsed(started_at);"));
}

#[test]
fn specialized_time_values_cross_supported_container_and_task_boundaries() {
    let program = parse_ok(
        r#"
fn measure() returns Duration {
    return 2ms
}

Task(Duration) measurement_task = run measure()
new Duration measured = wait measurement_task
new Duration List samples = [1ms, measured]
Channel(Duration) durations = channel(1)
durations.send(measured)
new Duration received = durations.receive()
"#,
    );
    semantic_analyze(&program).expect("semantic specialized time boundary source");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkadiList_duration samples"));
    assert!(c.contains("sk_channel_send_Duration"));
    assert!(c.contains("sk_channel_receive_Duration"));
    assert!(c.contains("int64_t result;"));
}
