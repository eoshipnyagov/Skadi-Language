use v01::formatter::format_source;

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
    let expected = r#"fn add(Int a, b) Int {
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
