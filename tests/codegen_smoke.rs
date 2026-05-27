use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

#[test]
fn codegen_emits_main_and_assignment() {
    let src = "new x = 1 + 2\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int main(void)"));
    assert!(c.contains("int64_t x = (1 + 2);"));
}

#[test]
fn codegen_emits_function_signature() {
    let src = r#"
fn add(Int a, Int b) Int {
    new c = a + b
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t add(int64_t a, int64_t b)"));
}

#[test]
fn codegen_emits_control_flow_and_return() {
    let src = r#"
fn f(x) {
    if x {
        new y = 1
    } else {
        y = 2
    }
    while y {
        y = y - 1
    }
    return y
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("if (x) {"));
    assert!(c.contains("while (y) {"));
    assert!(c.contains("return y;"));
}

#[test]
fn codegen_respects_typed_new() {
    let src = "new Float temperature = 21.5\n";
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("double temperature = 21.5;"));
}

#[test]
fn codegen_emits_danger_on_error_shape() {
    let src = r#"
new x = 0
x = parse_value(x) on error {
    x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("if (parse_value(x, &x) != 0) {"));
    assert!(!c.contains("TODO(v1): danger call lowering"));
}

#[test]
fn codegen_emits_danger_fn_with_out_param() {
    let src = r#"
danger fn parse_value(Int x) Int {
    return x
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int parse_value(int64_t x, int64_t *out)"));
    assert!(c.contains("*out = x;"));
}

#[test]
fn codegen_emits_error_status_for_empty_return_in_danger_fn() {
    let src = r#"
danger fn parse_value(Int x) Int {
    return
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int parse_value(int64_t x, int64_t *out)"));
    assert!(c.contains("return 1;"));
}

#[test]
fn codegen_emits_error_enum_and_return_error() {
    let src = r#"
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn parse_value(Int x) Int {
    return error ZeroDivision
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("typedef enum ErrorCode"));
    assert!(c.contains("ErrorCode_ZeroDivision = 1"));
    assert!(c.contains("return ErrorCode_ZeroDivision;"));
}

#[test]
fn codegen_emits_regular_call_expression() {
    let src = r#"
fn add(Int a, Int b) Int {
    return a + b
}
new Int x = 1
new Int y = add(x, 2)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t y = add(x, 2);"));
}

#[test]
fn codegen_lowers_when_to_if_chain() {
    let src = r#"
new Int x = 2
when x {
    is 1 {
        new y = 10
    }
    is 2, 3 {
        new y = 20
    }
    else {
        new y = 0
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t __when_tmp_1 = x;"));
    assert!(c.contains("if ((__when_tmp_1 == 1)) {"));
    assert!(c.contains("else if ((__when_tmp_1 == 2) || (__when_tmp_1 == 3)) {"));
    assert!(c.contains("else {"));
}

#[test]
fn codegen_emits_text_runtime_for_when_find_expression() {
    let src = r#"
new Text t = "alpha"
when find(t, "a") {
    is -1 {
        new Int x = 0
    }
    else {
        new Int x = 1
    }
}
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    assert!(c.contains("static int64_t sk_text_find(const char *s, const char *needle) {"));
    assert!(c.contains("int64_t __when_tmp_1 = sk_text_find(t, \"a\");"));
}

#[test]
fn codegen_lowers_for_in_to_list_iteration_shape() {
    let src = r#"
new Int sum = 0
for item in items {
    sum = sum + item
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("for (size_t __i = 0; __i < items.len; ++__i) {"));
    assert!(c.contains("int64_t item = items.data[__i];"));
}

#[test]
fn codegen_lowers_for_in_with_typed_list_element() {
    let src = r#"
new u8 List items = [1, 2, 3]
for item in items {
    new Int x = 1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("for (size_t __i = 0; __i < items.len; ++__i) {"));
    assert!(c.contains("uint8_t item = items.data[__i];"));
}

#[test]
fn codegen_emits_list_runtime_push_pop_shape() {
    let src = r#"
new i32 List xs = [1, 2]
new i32 x = 0
xs.push(3)
x = xs.pop() on error {
    x = 0
}
new Int n = len(xs)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("typedef struct {"));
    assert!(c.contains("SkadiList_i32 xs = sk_list_i32_new();"));
    assert!(c.contains("sk_list_i32_push(&xs, 3)"));
    assert!(c.contains("if (sk_list_i32_pop(&xs, &x) != 0) {"));
    assert!(c.contains("int64_t n = ((int64_t)xs.len);"));
}

#[test]
fn codegen_emits_list_runtime_for_multiple_scalar_types() {
    let src = r#"
new u8 List bu = [1, 2]
new f64 List fd = [1.0, 2.0]
new bool List bb = [true, false]
bu.push(3)
fd.push(3.5)
bb.push(true)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkadiList_u8 bu = sk_list_u8_new();"));
    assert!(c.contains("SkadiList_f64 fd = sk_list_f64_new();"));
    assert!(c.contains("SkadiList_bool bb = sk_list_bool_new();"));
    assert!(c.contains("sk_list_u8_push(&bu, 3)"));
    assert!(c.contains("sk_list_f64_push(&fd, 3.5)"));
    assert!(c.contains("sk_list_bool_push(&bb, true)"));
}

#[test]
fn codegen_emits_text_len_and_index_shape() {
    let src = r#"
new Text t = "weather"
new Int n = len(t)
new char c = t[0]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("const char* t = \"weather\";"));
    assert!(c.contains("int64_t n = ((int64_t)strlen(t));"));
    assert!(c.contains("char c = sk_text_char_at(t, 0);"));
}

#[test]
fn codegen_emits_text_contains_find_slice_shape() {
    let src = r#"
new Text t = "weather station"
new bool has = contains(t, "station")
new Int idx = find(t, "ther")
new Text tail = slice(t, 3, 7)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("bool has = (strstr(t, \"station\") != NULL);"));
    assert!(c.contains("int64_t idx = sk_text_find(t, \"ther\");"));
    assert!(c.contains("const char* tail = sk_text_slice(t, 3, 7);"));
}

#[test]
fn codegen_emits_fs_list_and_is_dir_shape() {
    let src = r#"
new Text root = "."
new Text List entries = fs.list(root)
new bool d = fs.is_dir(root)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkadiList_text entries = sk_list_text_new();"));
    assert!(c.contains("entries = sk_fs_list(root);"));
    assert!(c.contains("bool d = sk_fs_is_dir(root);"));
}

#[test]
fn codegen_lowers_path_list_to_text_list_runtime_shape() {
    let src = r#"
new Path List entries = fs.list(".")
new Path first = entries[0]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("SkadiList_text entries = sk_list_text_new();"));
    assert!(c.contains("entries = sk_fs_list(\".\");"));
    assert!(c.contains("const char* first = sk_list_text_get(&entries, 0);"));
}

#[test]
fn codegen_emits_output_input_read_write_shape() {
    let src = r#"
output("hello")
new Text name = input("name: ")
new Text body = read("in.txt")
new Int ok = write("out.txt", body)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_output_text(\"hello\");"));
    assert!(c.contains("const char* name = sk_input(\"name: \");"));
    assert!(c.contains("const char* body = sk_read_file(\"in.txt\");"));
    assert!(c.contains("int64_t ok = sk_write_file(\"out.txt\", body);"));
}

#[test]
fn codegen_emits_args_and_fs_join_shape() {
    let src = r#"
new Text List cli_args = args()
new Text p = fs.join(".", "src")
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int main(int argc, char **argv) {"));
    assert!(c.contains("SkadiList_text cli_args = sk_list_text_new();"));
    assert!(c.contains("cli_args = sk_args(argc, argv);"));
    assert!(c.contains("const char* p = sk_fs_join(\".\", \"src\");"));
}

#[test]
fn codegen_emits_concat_and_text_compare_shape() {
    let src = r#"
new Text a = "ab"
new Text b = "cd"
new Text c = concat(a, b)
if c == "abcd" {
    output(c)
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("const char* c = sk_text_concat(a, b);"));
    assert!(c.contains("if ((strcmp(c, \"abcd\") == 0)) {"));
}

#[test]
fn codegen_emits_struct_typedef_and_typed_literal() {
    let src = r#"
struct Sensor {
    Int id
    Text name
}

new Sensor s = {id = 7, name = "cpu"}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("typedef struct {"));
    assert!(c.contains("int64_t id;"));
    assert!(c.contains("const char* name;"));
    assert!(c.contains("} Sensor;"));
    assert!(c.contains("Sensor s = (Sensor){.id = 7, .name = \"cpu\"};"));
}

#[test]
fn codegen_emits_struct_method_with_my_and_call_lowering() {
    let src = r#"
struct Counter {
    Int value
    fn inc(Int delta) Int {
        my.value = my.value + delta
        return my.value
    }
}

new Counter c = {value = 1}
new Int next = c.inc(2)
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("int64_t Counter_inc(Counter *my, int64_t delta)"));
    assert!(c.contains("my->value = (my->value + delta);"));
    assert!(c.contains("int64_t next = Counter_inc(&c, 2);"));
}

#[test]
fn codegen_emits_struct_list_runtime_shape() {
    let src = r#"
struct Account {
    Int balance
}

new Account List accounts = []
new Account a = {balance = 10}
accounts.push(a)
new Account first = accounts[0]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("typedef struct {"));
    assert!(c.contains("} SkadiList_Account;"));
    assert!(c.contains("SkadiList_Account accounts = sk_list_Account_new();"));
    assert!(c.contains("sk_list_Account_push(&accounts, a)"));
    assert!(c.contains("Account first = sk_list_Account_get(&accounts, 0);"));
}

#[test]
fn codegen_list_index_runtime_contract_is_fail_soft_zero() {
    let src = r#"
new i32 List xs = [1]
new i32 v = xs[999]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("static int32_t sk_list_i32_get(const SkadiList_i32 *xs, int64_t idx) {"));
    assert!(c.contains("if (!xs || idx < 0 || (size_t)idx >= xs->len) {"));
    assert!(c.contains("memset(&z, 0, sizeof(z));"));
    assert!(c.contains("return z;"));
}

#[test]
fn codegen_text_index_runtime_contract_is_fail_soft_nul() {
    let src = r#"
new Text t = "ab"
new char c = t[999]
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("static char sk_text_char_at(const char *s, int64_t idx) {"));
    assert!(c.contains("if (!s || idx < 0) return '\\0';"));
    assert!(c.contains("if ((size_t)idx >= n) return '\\0';"));
}

#[test]
fn codegen_emits_text_runtime_for_find_in_if_condition() {
    let src = r#"
new Text t = "alpha"
if find(t, "ph") >= 0 {
    output("ok")
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("static int64_t sk_text_find(const char *s, const char *needle) {"));
    assert!(c.contains("if ((sk_text_find(t, \"ph\") >= 0)) {"));
}

#[test]
fn codegen_lowers_text_list_index_equality_with_strcmp() {
    let src = r#"
new Text List keys = ["a"]
new Text key = "a"
if keys[0] == key {
    output(1)
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("if ((strcmp(sk_list_text_get(&keys, 0), key) == 0)) {"));
}

#[test]
fn codegen_routes_output_of_text_list_index_to_text_output() {
    let src = r#"
new Text List keys = ["a"]
output(keys[0])
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("sk_output_text(sk_list_text_get(&keys, 0));"));
}

#[test]
fn codegen_emits_fs_runtime_for_fs_calls_inside_when() {
    let src = r#"
new Text p = "."
when fs.is_dir(p) {
    is true {
        output(1)
    }
    else {
        output(0)
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("static bool sk_fs_is_dir(const char *path) {"));
    assert!(c.contains("int64_t __when_tmp_1 = sk_fs_is_dir(p);"));
}

#[test]
fn codegen_emits_text_runtime_for_text_return_expression() {
    let src = r#"
fn tail(Text t) Text {
    return slice(t, 1, len(t))
}
new Text x = tail("abc")
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("static char* sk_text_slice(const char *s, int64_t start, int64_t end) {"));
    assert!(c.contains("return sk_text_slice(t, 1, ((int64_t)strlen(t)));"));
}

#[test]
fn codegen_emits_break_continue_pass_and_inc_dec_shape() {
    let src = r#"
new Int i = 0
loop {
    i++
    pass
    if i > 2 {
        break
    } else {
        continue
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("i += 1;"));
    assert!(c.contains("/* pass */"));
    assert!(c.contains("break;"));
    assert!(c.contains("continue;"));
}

#[test]
fn codegen_invariant_when_lowering_uses_single_temp() {
    let src = r#"
new Int x = 3
when x {
    is 1 {
        output(1)
    }
    is 2, 3 {
        output(2)
    }
    else {
        output(0)
    }
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.matches("__when_tmp_1").count() >= 4);
    assert!(c.contains("if ((__when_tmp_1 == 1)) {"));
    assert!(c.contains("else if ((__when_tmp_1 == 2) || (__when_tmp_1 == 3)) {"));
}

#[test]
fn codegen_invariant_danger_on_error_emits_status_check_branch() {
    let src = r#"
label ErrorCode {
    Ok
    BadInput
}

danger fn parse_nonzero(Int x) Int {
    if x == 0 {
        return error BadInput
    } else {
        return x
    }
}

new Int value = 0
new Int parsed = 0
parsed = parse_nonzero(value) on error {
    parsed = -1
}
"#;
    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);
    assert!(c.contains("if (parse_nonzero(value, &parsed) != 0) {"));
    assert!(c.contains("parsed = (-1);") || c.contains("parsed = -1;"));
}
