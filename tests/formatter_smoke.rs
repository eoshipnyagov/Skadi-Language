use v01::formatter::format_source;
use v01::lexer::lex;
use v01::parser::parse_program;

#[test]
fn formats_functions_control_flow_and_expressions() {
    let source = r#"
fn  add( Int a,b) Int{
new sum= a+ b*2
if sum>10{
output("big")
}else{
output("small")
}
return sum
}
"#;

    let formatted = format_source(source).expect("format should succeed");
    let expected = r#"fn add(Int a, b) returns Int {
    new sum = a + b * 2
    if sum > 10 {
        output("big")
    } else {
        output("small")
    }
    return sum
}
"#;

    assert_eq!(formatted, expected);
}

#[test]
fn formats_structs_when_blocks_and_danger_on_error() {
    let source = r#"
struct Sensor{
Text name
Int id
danger fn read_value(){
value = read("sensor.txt") on error{
return error IO_FAIL
}
}
}

when state{
is Ready,Idle{
output("go")
}
else{
output("wait")
}
}
"#;

    let formatted = format_source(source).expect("format should succeed");
    let expected = r#"struct Sensor {
    Text name
    Int id

    danger fn read_value() {
        value = read("sensor.txt") on error {
            return error IO_FAIL
        }
    }
}

when state {
    is Ready, Idle {
        output("go")
    }

    else {
        output("wait")
    }
}
"#;

    assert_eq!(formatted, expected);
}

#[test]
fn formats_on_blocks_and_legacy_for_loops() {
    let source = r#"
on interrupt  timer0{
new Int ticks=0
}

for(i=0; i<10; i++){
output(i)
}
"#;

    let formatted = format_source(source).expect("format should succeed");
    let expected = r#"on interrupt timer0 {
    new Int ticks = 0
}

for (i = 0; i < 10; i++) {
    output(i)
}
"#;

    assert_eq!(formatted, expected);
}

#[test]
fn formats_view_and_edit_borrows_distinctly() {
    let source = r#"
fn inspect(view Canvas frame)returns Int{
return frame.checksum()
}
fn clear(edit Canvas frame){
frame.clear(Color.black)
}
Canvas frame=canvas(8,8)
new Int checksum=inspect(view frame)
clear(edit frame)
"#;

    let formatted = format_source(source).expect("format should support view borrows");
    assert!(formatted.contains("fn inspect(view Canvas frame) returns Int"));
    assert!(formatted.contains("inspect(view frame)"));
    assert!(formatted.contains("fn clear(edit Canvas frame)"));
    assert!(formatted.contains("clear(edit frame)"));
}

#[test]
fn owning_resource_declarations_survive_format_parse_roundtrip() {
    let source = r#"
fn worker(){pass}
Channel(Int) jobs=channel(1)
Task worker_task=run worker()
Canvas frame=canvas(8,8)
Window window=windows.open("Skadi",8,8)
Interrupt timer=interrupts.periodic(10ms)
wait worker_task
"#;

    let formatted = format_source(source).expect("format resources");
    assert!(formatted.contains("Channel(Int) jobs = channel(1)"));
    assert!(formatted.contains("Task worker_task = run worker()"));
    assert!(formatted.contains("Canvas frame = canvas(8, 8)"));
    assert!(formatted.contains("Window window = windows.open(\"Skadi\", 8, 8)"));
    assert!(formatted.contains("Interrupt timer = interrupts.periodic(10ms)"));
    assert!(!formatted.contains("new Channel"));
    assert!(!formatted.contains("new Task"));

    let tokens = lex(&formatted).expect("formatted resources should lex");
    parse_program(&tokens).expect("formatted resources should parse");
}

#[test]
fn formats_timed_channel_handlers_and_context_status() {
    let source = r#"
Channel(Int) jobs=channel(1)
new Int value=0
value=jobs.receive_for(25ms) on error{
if timed_out{value=-1}
}
jobs.send_for(7,25ms) on error{pass}
"#;

    let formatted = format_source(source).expect("format timed channels");
    assert!(formatted.contains("value = jobs.receive_for(25ms) on error {"));
    assert!(formatted.contains("if timed_out {"));
    assert!(formatted.contains("jobs.send_for(7, 25ms) on error {"));
    let tokens = lex(&formatted).expect("formatted timed channels should lex");
    parse_program(&tokens).expect("formatted timed channels should parse");
}

#[test]
fn formats_timed_task_wait_and_preserves_handler() {
    let source = r#"
fn worker(){sleep(20ms)}
Task worker_task=run worker()
wait worker_task for 1ms on error{
if timed_out{output("deadline")}
stop worker_task
wait worker_task
}
"#;

    let formatted = format_source(source).expect("format timed task wait");
    assert!(formatted.contains("wait worker_task for 1ms on error {"));
    assert!(formatted.contains("if timed_out {"));
    let tokens = lex(&formatted).expect("formatted timed task wait should lex");
    parse_program(&tokens).expect("formatted timed task wait should parse");
}

#[test]
fn formats_variadic_output_without_changing_argument_spacing() {
    let source = "output(\"count: \",count,\", ready=\",true)\n";
    let formatted = format_source(source).expect("format should succeed");
    assert_eq!(
        formatted,
        "output(\"count: \", count, \", ready=\", true)\n"
    );
}
