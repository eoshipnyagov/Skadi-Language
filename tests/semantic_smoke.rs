use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

#[test]
fn semantic_passes_for_defined_variables() {
    let src = "new a = 1\nnew b = a + 2\nb = b + 1\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_fails_for_use_before_definition() {
    let src = "b = a + 2\nnew a = 1\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("line"));
    assert!(err.contains("col"));
    assert!(err.contains("SC-SEM-012"));
    assert!(err.contains("use-before-definition"));
    assert!(err.contains("b"));
}

#[test]
fn semantic_fails_for_redeclaration_in_same_scope() {
    let src = "new a = 1\nnew a = 2\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-010"));
    assert!(err.contains("redeclaration in same scope"));
    assert!(err.contains("a"));
}

#[test]
fn semantic_fails_for_self_reference_on_initialization() {
    let src = "new x = x + 1\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-011"));
    assert!(err.contains("invalid initialization"));
    assert!(err.contains("x"));
}

#[test]
fn semantic_fails_for_on_error_without_danger_call_binding() {
    let src = r#"
on error {
    new x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("on error"));
}

#[test]
fn semantic_allows_on_interrupt_block() {
    let src = r#"
on interrupt timer0 {
    new x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_fails_for_undefined_variable_in_return() {
    let src = r#"
fn f() {
    return z
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("use-before-definition"));
    assert!(err.contains("z"));
}

#[test]
fn semantic_allows_danger_on_error_assignment_for_defined_target() {
    let src = r#"
danger fn parse_value(bool x) Int {
    if x {
        return 1
    } else {
        return 0
    }
}

new bool x = true
x = parse_value(x) on error {
    x = false
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_on_error_for_non_danger_function() {
    let src = r#"
fn parse_value(bool x) Int {
    if x {
        return 1
    } else {
        return 0
    }
}

new bool x = true
x = parse_value(x) on error {
    x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("on error requires danger fn call"));
    assert!(err.contains("parse_value"));
}

#[test]
fn semantic_allows_return_error_with_errorcode_label() {
    let src = r#"
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn parse_value(bool x) Int {
    return error ZeroDivision
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_unknown_return_error_code() {
    let src = r#"
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn parse_value(bool x) Int {
    return error InvalidFormat
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-051"));
    assert!(err.contains("unknown ErrorCode variant"));
}

#[test]
fn semantic_rejects_errorcode_without_ok_first() {
    let src = r#"
label ErrorCode {
    ZeroDivision
    Ok
}

danger fn parse_value(bool x) Int {
    return error ZeroDivision
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("must start with 'Ok'"));
}

#[test]
fn semantic_rejects_danger_fn_without_explicit_return() {
    let src = r#"
danger fn parse_value(bool x) Int {
    if x {
        return 1
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("must end with explicit return"));
}

#[test]
fn semantic_rejects_assignment_type_mismatch_bool_to_int() {
    let src = "new Int x = 1\nx = true\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("type mismatch in assignment"));
}

#[test]
fn semantic_allows_int_to_float_widening() {
    let src = "new Float x = 1\nx = 2\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_non_bool_if_condition() {
    let src = r#"
new Int x = 1
if x {
    x = 2
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("if condition must be bool"));
}

#[test]
fn semantic_rejects_danger_on_error_arg_type_mismatch() {
    let src = r#"
danger fn parse_value(bool x) Int {
    if x {
        return 1
    } else {
        return 0
    }
}
new Int x = 1
x = parse_value(x) on error {
    x = 0
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-032"));
    assert!(err.contains("argument type mismatch"));
}

#[test]
fn semantic_rejects_regular_call_arg_count_mismatch() {
    let src = r#"
fn add(Int a, Int b) Int {
    return a + b
}
new Int x = 1
new Int y = add(x)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-031"));
    assert!(err.contains("argument count mismatch"));
}

#[test]
fn semantic_rejects_when_case_type_mismatch() {
    let src = r#"
new Int x = 1
when x {
    is true {
        x = 2
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("type mismatch in when-case"));
}

#[test]
fn semantic_allows_typed_list_literal_and_len() {
    let src = r#"
new i32 List xs = [1, 2, 3]
new Int n = len(xs)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_mixed_type_list_literal() {
    let src = r#"
new i32 List xs = [1, true]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("type mismatch in list literal"));
}

#[test]
fn semantic_rejects_len_on_non_list_non_text() {
    let src = r#"
new Int x = 1
new Int n = len(x)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("builtin 'len' expects List or Text"));
}

#[test]
fn semantic_allows_list_index_access() {
    let src = r#"
new i32 List xs = [1, 2, 3]
new i32 x = xs[1]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_non_int_index_access() {
    let src = r#"
new i32 List xs = [1, 2, 3]
new i32 x = xs[true]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("index access requires Int index"));
}

#[test]
fn semantic_allows_list_push_and_pop_on_error() {
    let src = r#"
new i32 List xs = [1, 2]
new i32 x = 0
xs.push(3)
x = xs.pop() on error {
    x = 0
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_list_push_type_mismatch() {
    let src = r#"
new i32 List xs = [1, 2]
xs.push(true)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("type mismatch in list push"));
}

#[test]
fn semantic_allows_text_len_and_index() {
    let src = r#"
new Text t = "weather"
new Int n = len(t)
new char c = t[0]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_allows_text_contains_find_slice() {
    let src = r#"
new Text t = "weather station"
new bool has = contains(t, "station")
new Int idx = find(t, "ther")
new Text tail = slice(t, 3, 7)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_text_builtin_type_mismatch() {
    let src = r#"
new Text t = "weather"
new bool has = contains(t, 1)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("builtin 'contains' expects (Text, Text)"));
}

#[test]
fn semantic_rejects_contains_wrong_arg_count() {
    let src = r#"
new Text t = "weather"
new bool has = contains(t)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-033"));
    assert!(err.contains("builtin 'contains' expects 2 arguments"));
}

#[test]
fn semantic_rejects_find_wrong_arg_count() {
    let src = r#"
new Text t = "weather"
new Int idx = find(t)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-033"));
    assert!(err.contains("builtin 'find' expects 2 arguments"));
}

#[test]
fn semantic_rejects_find_type_mismatch() {
    let src = r#"
new Text t = "weather"
new Int idx = find(t, 10)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("builtin 'find' expects (Text, Text)"));
}

#[test]
fn semantic_rejects_slice_wrong_arg_count() {
    let src = r#"
new Text t = "weather"
new Text s = slice(t, 1)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-033"));
    assert!(err.contains("builtin 'slice' expects 3 arguments"));
}

#[test]
fn semantic_rejects_slice_type_mismatch() {
    let src = r#"
new Text t = "weather"
new Text s = slice(t, true, 3)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("builtin 'slice' expects (Text, Int, Int)"));
}

#[test]
fn semantic_allows_slice_with_start_greater_than_end() {
    let src = r#"
new Text t = "weather"
new Text s = slice(t, 5, 2)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_for_infers_item_type_from_list() {
    let src = r#"
new i32 List samples = [10, 20, 30]
new Int sum = 0
for item in samples {
    sum = sum + item
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_for_over_non_collection() {
    let src = r#"
new Int x = 1
for item in x {
    x = x + 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("expects List or Text collection"));
}
