use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

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

#[test]
fn semantic_allows_fs_list_and_is_dir_builtins() {
    let src = r#"
new Text root = "."
new Text List entries = fs.list(root)
new bool d = fs.is_dir(root)
for e in entries {
    d = fs.is_dir(e)
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_fs_list_non_text_argument() {
    let src = r#"
new Int x = 1
new Text List entries = fs.list(x)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("builtin 'fs.list' expects (Text)"));
}

#[test]
fn semantic_allows_io_builtins() {
    let src = r#"
output("hello")
new Text name = input("name: ")
new Text body = read("a.txt")
new Int ok = write("b.txt", body)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_allows_args_and_fs_join() {
    let src = r#"
new Text List a = args()
new Text p = fs.join(".", "src")
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_allows_struct_literal_field_punning_for_defined_vars() {
    let source = r#"
new Int value = 10
new Int status = 1
new result = {value, status}
"#;
    let tokens = v01::lexer::lex(source).expect("lexing should succeed");
    let program = v01::parser::parse_program(&tokens).expect("parsing should succeed");
    let result = v01::semantic_analysis::semantic_analyze(&program);
    assert!(result.is_ok(), "expected semantic success, got: {:?}", result);
}

#[test]
fn semantic_rejects_my_outside_struct_method() {
    let src = r#"
new Int x = 1
new Int y = my.value
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-040"));
    assert!(err.contains("my is only allowed inside struct methods"));
}

#[test]
fn semantic_rejects_unknown_struct_method_call() {
    let src = r#"
struct Counter {
    Int value
    fn inc(Int d) Int {
        my.value = my.value + d
        return my.value
    }
}
new Counter c = {value = 1}
new Int y = c.dec(1)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-030"));
    assert!(err.contains("unknown method 'Counter.dec'"));
}

#[test]
fn semantic_accepts_struct_list_push_and_index() {
    let src = r#"
struct Account {
    Int balance
}
new Account List accounts = []
new Account a = {balance = 1}
accounts.push(a)
new Account x = accounts[0]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_wrong_type_push_into_struct_list() {
    let src = r#"
struct Account {
    Int balance
}
new Account List accounts = []
accounts.push(1)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("type mismatch in list push"));
}

#[test]
fn style_warnings_allow_user_defined_struct_types() {
    let src = r#"
struct Sensor {
    Int value
}
new Sensor s = {value = 1}
new Sensor List xs = []
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let warnings = semantic_style_warnings(&program);
    assert!(
        warnings
            .iter()
            .all(|w| !w.contains("non-canonical type spelling 'Sensor'")),
        "unexpected warning list: {:?}",
        warnings
    );
}

#[test]
fn semantic_allows_concat_text_builtin() {
    let src = r#"
new Text a = "ab"
new Text b = "cd"
new Text c = concat(a, b)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn style_warnings_prefer_iterate_over_for_in_showcase() {
    let src = r#"
new i32 List xs = [1, 2]
for item in xs {
    new Int x = item
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let warnings = semantic_style_warnings(&program);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("prefer 'iterate <collection> as <item>'")),
        "expected iterate-style warning, got: {:?}",
        warnings
    );
}

#[test]
fn style_warnings_prefer_bool_and_char_canonical_names() {
    let src = r#"
new bool ok = true
new char ch = 'a'
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let warnings = semantic_style_warnings(&program);
    assert!(
        warnings.iter().any(|w| w.contains("prefer 'Bool' over 'bool'")),
        "expected Bool warning, got: {:?}",
        warnings
    );
    assert!(
        warnings.iter().any(|w| w.contains("prefer 'Char' over 'char'")),
        "expected Char warning, got: {:?}",
        warnings
    );
}

#[test]
fn semantic_rejects_on_error_for_read_builtin() {
    let src = r#"
new Text body = ""
body = read("a.txt") on error {
    body = ""
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("on error requires danger fn call"));
    assert!(err.contains("read"));
}

#[test]
fn semantic_rejects_on_error_for_write_builtin() {
    let src = r#"
new Int ok = 0
ok = write("out.txt", "x") on error {
    ok = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("on error requires danger fn call"));
    assert!(err.contains("write"));
}

#[test]
fn semantic_rejects_on_error_for_output_builtin() {
    let src = r#"
output("hello") on error {
    new Int x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("on error requires danger fn call"));
    assert!(err.contains("output"));
}

#[test]
fn semantic_allows_increment_for_numeric_variable() {
    let src = r#"
new Int i = 0
i++
i--
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic analysis should pass");
}

#[test]
fn semantic_rejects_increment_for_non_numeric_variable() {
    let src = r#"
new Text s = "x"
s++
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
    assert!(err.contains("SC-SEM-020"));
    assert!(err.contains("increment/decrement requires numeric variable"));
}
