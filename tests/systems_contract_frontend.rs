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
fn nominal_values_flow_through_functions_and_short_when_cases() {
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

fn choose(Direction direction) returns ExitCode {
    when direction {
        is North {
            return ExitCode.Success
        }
        is South {
            return ExitCode.InvalidInput
        }
    }
    return ExitCode.InvalidInput
}

new Direction direction = Direction.North
new ExitCode result = choose(direction)
new Bool accepted = result == ExitCode.Success
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("ExitCode choose(Direction direction)"), "{c}");
    assert!(c.contains("Direction_North"), "{c}");
    assert!(c.contains("Direction_South"), "{c}");
    assert!(c.contains("ExitCode result = choose(direction)"), "{c}");
}

#[test]
fn short_when_case_must_belong_to_the_subject_nominal_type() {
    let err = semantic_err(
        r#"
tag Direction {
    North
    South
}

new Direction direction = Direction.North
when direction {
    is Missing {
        pass
    }
}
"#,
    );
    assert!(err.contains("SC-SEM-020"), "{err}");
    assert!(
        err.contains("variant 'Missing' does not belong to tag 'Direction'"),
        "{err}"
    );
}

#[test]
fn variadic_output_accepts_printable_values_and_rejects_empty_calls() {
    semantic_ok(
        r#"
new Text name = "Skadi"
new Int count = 3
new Bool ready = true
output(name, ": ", count, ", ready=", ready)
"#,
    );

    let empty = semantic_err("output()\n");
    assert!(empty.contains("SC-SEM-033"), "{empty}");
    assert!(empty.contains("expects at least 1 argument"), "{empty}");
}

#[test]
fn fixed_and_const_are_available_as_ordinary_identifiers() {
    semantic_ok(
        r#"
new Int fixed = 4
new Int const = 5
output("sum=", fixed + const)
"#,
    );
}

#[test]
fn value_edit_and_view_parameters_enforce_the_call_contract() {
    let program = semantic_ok(
        r#"
fn increment(edit Int value) {
    value += 1
}

fn observe(view Int value) Int {
    return value
}

constant Int answer = 42
new Int count = 1
increment(edit count)
new Int observed = observe(view answer)
"#,
    );
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkInt increment(SkInt * value)"), "{c}");
    assert!(c.contains("SkInt observe(const SkInt * value)"), "{c}");
    assert!(c.contains("increment(&count)"), "{c}");
    assert!(c.contains("const SkInt answer = 42"), "{c}");

    let missing_edit = semantic_err(
        r#"
fn increment(edit Int value) {
    value += 1
}

new Int count = 1
increment(count)
"#,
    );
    assert!(missing_edit.contains("requires explicit 'edit <identifier>'"));

    let edits_constant = semantic_err(
        r#"
fn increment(edit Int value) {
    value += 1
}

constant Int answer = 42
increment(edit answer)
"#,
    );
    assert!(
        edits_constant.contains("constant binding 'answer' cannot be borrowed with 'edit'"),
        "{edits_constant}"
    );

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
new Int observed = observe(edit value)
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
fn increment(edit Int value) {
    value += 1
}

new Int count = 1
Task worker = run increment(edit count)
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
fn obsolete_direct_spelling_is_not_a_borrow_marker() {
    let tokens = lex("direct").expect("lex obsolete borrow spelling");
    assert_eq!(tokens[0].kind(), v01::common_types::TokenKind::Identifier);
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
