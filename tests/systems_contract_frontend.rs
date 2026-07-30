use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn parse_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex source");
    parse_program(&tokens).expect("parse source")
}

fn parse_err(source: &str) -> String {
    let tokens = lex(source).expect("lex source");
    parse_program(&tokens).expect_err("expected parse error")
}

fn semantic_ok(source: &str) -> v01::ast_nodes::Program {
    let program = parse_ok(source);
    semantic_analyze(&program).expect("semantic analysis");
    program
}

fn semantic_err(source: &str) -> String {
    let program = parse_ok(source);
    semantic_analyze(&program).expect_err("expected semantic error")
}

#[test]
fn label_and_tag_are_distinct_nominal_types() {
    semantic_ok(
        r#"
label ExitCode {
    Success = 0
    InvalidInput = 64
}

tag Direction {
    North
    South
}

new ExitCode exit_code = ExitCode.Success
new Direction direction = Direction.North
"#,
    );

    let missing_discriminant = parse_err(
        r#"
label ExitCode {
    Success
}
"#,
    );
    assert!(missing_discriminant.contains("SC-PARSE-"));
    assert!(missing_discriminant.contains("requires explicit integer discriminant"));

    let duplicate_discriminant = semantic_err(
        r#"
label ExitCode {
    Success = 0
    AlsoSuccess = 0
}
"#,
    );
    assert!(duplicate_discriminant.contains("duplicate discriminant 0"));
}

#[test]
fn codegen_emits_explicit_labels_and_symbolic_tags() {
    let program = semantic_ok(
        r#"
label ExitCode {
    Success = 0
    InvalidInput = 64
}

tag Direction {
    North
    South
}

new ExitCode exit_code = ExitCode.InvalidInput
new Direction direction = Direction.South
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("ExitCode_InvalidInput = 64"));
    assert!(c.contains("Direction_North = 0"));
    assert!(c.contains("Direction_South = 1"));
}

#[test]
fn value_direct_and_view_parameters_enforce_the_call_contract() {
    let program = semantic_ok(
        r#"
fn increment(direct Int value) {
    value += 1
}

fn observe(view Int value) Int {
    return value
}

constant Int answer = 42
new Int count = 1
increment(direct count)
new Int observed = observe(view answer)
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t increment(int64_t * value)"), "{c}");
    assert!(c.contains("int64_t observe(const int64_t * value)"), "{c}");
    assert!(c.contains("increment(&count)"), "{c}");
    assert!(c.contains("const int64_t answer = 42"), "{c}");

    let missing_direct = semantic_err(
        r#"
fn increment(direct Int value) {
    value += 1
}

new Int count = 1
increment(count)
"#,
    );
    assert!(missing_direct.contains("requires explicit 'direct <identifier>'"));

    let mutates_read_only = semantic_err(
        r#"
fn overwrite(view Int value) {
    value = 2
}
"#,
    );
    assert!(
        mutates_read_only.contains("view parameter 'value' cannot be reassigned"),
        "{mutates_read_only}"
    );

    let wrong_view_marker = semantic_err(
        r#"
fn observe(view Int value) returns Int {
    return value
}

new Int value = 1
new Int observed = observe(direct value)
"#,
    );
    assert!(
        wrong_view_marker.contains("requires explicit 'view <identifier>'"),
        "{wrong_view_marker}"
    );
}

#[test]
fn call_scoped_borrow_cannot_cross_a_task_boundary() {
    let err = semantic_err(
        r#"
fn increment(direct Int value) {
    value += 1
}

new Int count = 1
Task worker = run increment(direct count)
wait worker
"#,
    );
    assert!(err.contains("call-scoped borrow"));
    assert!(err.contains("cannot cross the task boundary"));

    let view_err = semantic_err(
        r#"
fn observe(view Int value) {
    output(value)
}

constant Int answer = 42
Task worker = run observe(view answer)
wait worker
"#,
    );
    assert!(view_err.contains("call-scoped borrow"), "{view_err}");
}

#[test]
fn constant_direct_points_to_the_view_replacement() {
    let err = parse_err(
        r#"
fn observe(constant direct Int value) {
    output(value)
}
"#,
    );
    assert!(err.contains("SC-PARSE-106"), "{err}");
    assert!(err.contains("replaced by 'view <Type> <name>'"), "{err}");
}

#[test]
fn channel_owner_controls_close_and_try_send_is_fallible() {
    semantic_ok(
        r#"
Channel(Int) values = channel(2)
new Bool accepted = values.try_send(1)
values.close()
"#,
    );

    let borrowed_close = semantic_err(
        r#"
fn close_borrowed(Channel(Int) values) {
    values.close()
}
"#,
    );
    assert!(borrowed_close.contains("only owning Channel binding may close"));

    let double_close = semantic_err(
        r#"
Channel(Int) values = channel(2)
values.close()
values.close()
"#,
    );
    assert!(double_close.contains("already closed"));

    let maybe_closed_send = semantic_err(
        r#"
Channel(Int) values = channel(2)
new Bool should_close = true

if should_close {
    values.close()
}

values.send(1)
"#,
    );
    assert!(
        maybe_closed_send.contains("resource 'values' may be closed"),
        "{maybe_closed_send}"
    );

    semantic_ok(
        r#"
Channel(Int) values = channel(2)
new Bool should_close = true

if should_close {
    values.close()
}

values.send(1) on error {
    pass
}
"#,
    );
}

#[test]
fn channel_codegen_separates_plain_and_fallible_send() {
    let program = semantic_ok(
        r#"
Channel(Int) values = channel(2)
values.send(1)
values.close()
values.send(2) on error {
    output("closed")
}
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_channel_send_or_panic_Int(values, 1)"), "{c}");
    assert!(
        c.contains("if (sk_channel_send_Int(values, 2) != 0)"),
        "{c}"
    );
    assert!(c.contains("sk_channel_close(values)"), "{c}");
}

#[test]
fn channel_owner_can_be_moved_or_returned_without_affecting_shared_parameters() {
    let program = semantic_ok(
        r#"
fn make_channel() returns Channel(Int) {
    Channel(Int) values = channel(2)
    return move values
}

fn finish(move Channel(Int) values) {
    values.close()
}

Channel(Int) values = make_channel()
finish(move values)
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkChannel* make_channel("), "{c}");
    assert!(c.contains("sk_channel_move(&values)"), "{c}");
    assert!(c.contains("sk_channel_destroy(values)"), "{c}");

    let moved = semantic_err(
        r#"
fn finish(move Channel(Int) values) {
    values.close()
}

Channel(Int) values = channel(2)
finish(move values)
values.send(1) on error {
    pass
}
"#,
    );
    assert!(moved.contains("resource 'values' was moved"), "{moved}");
}

#[test]
fn window_and_interrupt_owners_use_the_shared_move_lowering() {
    let program = semantic_ok(
        r#"
fn make_window() returns Window {
    Window window = windows.open("Move", 32, 24)
    return move window
}

fn finish_window(move Window window) {
    window.close()
}

fn make_tick() returns Interrupt {
    Interrupt tick = interrupts.periodic(10ms)
    return move tick
}

fn finish_tick(move Interrupt tick) {
    pass
}

Window window = make_window()
finish_window(move window)
Interrupt tick = make_tick()
finish_tick(move tick)
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_window_move(&window)"), "{c}");
    assert!(c.contains("sk_interrupt_move(&tick)"), "{c}");
    assert!(c.contains("sk_window_destroy(&window)"), "{c}");
    assert!(c.contains("sk_interrupt_destroy(tick)"), "{c}");
}
