use std::collections::{HashMap, HashSet};

use crate::ast_nodes::{BlockStatement, Expression, Program, Statement};
use crate::builtins::{builtin_from_name, Builtin};

struct FunctionContext {
    is_danger: bool,
    return_type: Option<String>,
}

const LIST_TYPE_MAP: [(&str, &str, &str); 12] = [
    ("i8", "int8_t", "i8"),
    ("i16", "int16_t", "i16"),
    ("i32", "int32_t", "i32"),
    ("i64", "int64_t", "i64"),
    ("u8", "uint8_t", "u8"),
    ("u16", "uint16_t", "u16"),
    ("u32", "uint32_t", "u32"),
    ("u64", "uint64_t", "u64"),
    ("f32", "float", "f32"),
    ("f64", "double", "f64"),
    ("bool", "bool", "bool"),
    ("Text", "const char*", "text"),
];

fn list_elem_from_decl(t: &str) -> Option<&str> {
    t.strip_suffix(" List").map(str::trim)
}

fn list_meta(elem: &str) -> Option<(&'static str, &'static str)> {
    let normalized = match elem {
        "Int" => "i64",
        "Float" => "f64",
        "Bool" => "bool",
        "Char" => "char",
        "Path" => "Text",
        other => other,
    };
    LIST_TYPE_MAP
        .iter()
        .find(|(name, _, _)| *name == normalized)
        .map(|(_, c_ty, suffix)| (*c_ty, *suffix))
}

fn list_meta_dynamic(elem: &str) -> (String, String) {
    if let Some((c_ty, suffix)) = list_meta(elem) {
        return (c_ty.to_string(), suffix.to_string());
    }
    (elem.to_string(), elem.to_string())
}

fn collect_struct_names(program: &Program) -> Vec<String> {
    program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::StructDecl { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn emit_list_helpers_for(out: &mut String, c_ty: &str, suffix: &str) {
    out.push_str(&format!(
        "typedef struct {{\n    {} *data;\n    size_t len;\n    size_t cap;\n}} SkadiList_{};\n\n",
        c_ty, suffix
    ));
    out.push_str(&format!("static SkadiList_{} sk_list_{}_new(void) {{\n", suffix, suffix));
    out.push_str(&format!("    SkadiList_{} xs;\n", suffix));
    out.push_str("    xs.data = NULL;\n");
    out.push_str("    xs.len = 0;\n");
    out.push_str("    xs.cap = 0;\n");
    out.push_str("    return xs;\n");
    out.push_str("}\n\n");
    out.push_str(&format!(
        "static int sk_list_{}_push(SkadiList_{} *xs, {} v) {{\n",
        suffix, suffix, c_ty
    ));
    out.push_str("    if (xs->len == xs->cap) {\n");
    out.push_str("        size_t next = xs->cap == 0 ? 4 : xs->cap * 2;\n");
    out.push_str(&format!(
        "        {} *p = ({0}*)realloc(xs->data, next * sizeof({0}));\n",
        c_ty
    ));
    out.push_str("        if (!p) return 1;\n");
    out.push_str("        xs->data = p;\n");
    out.push_str("        xs->cap = next;\n");
    out.push_str("    }\n");
    out.push_str("    xs->data[xs->len++] = v;\n");
    out.push_str("    return 0;\n");
    out.push_str("}\n\n");
    out.push_str(&format!(
        "static int sk_list_{}_pop(SkadiList_{} *xs, {} *out) {{\n",
        suffix, suffix, c_ty
    ));
    out.push_str("    if (xs->len == 0) return 1;\n");
    out.push_str("    *out = xs->data[xs->len - 1];\n");
    out.push_str("    xs->len -= 1;\n");
    out.push_str("    return 0;\n");
    out.push_str("}\n\n");
    out.push_str(&format!(
        "static {} sk_list_{}_get(const SkadiList_{} *xs, int64_t idx) {{\n",
        c_ty, suffix, suffix
    ));
    out.push_str("    if (!xs || idx < 0 || (size_t)idx >= xs->len) {\n");
    out.push_str(&format!("        {} z;\n", c_ty));
    out.push_str("        memset(&z, 0, sizeof(z));\n");
    out.push_str("        return z;\n");
    out.push_str("    }\n");
    out.push_str("    return xs->data[(size_t)idx];\n");
    out.push_str("}\n\n");
    out.push_str(&format!(
        "static void sk_list_{}_free(SkadiList_{} *xs) {{\n",
        suffix, suffix
    ));
    out.push_str("    if (!xs) return;\n");
    out.push_str("    free(xs->data);\n");
    out.push_str("    xs->data = NULL;\n");
    out.push_str("    xs->len = 0;\n");
    out.push_str("    xs->cap = 0;\n");
    out.push_str("}\n\n");
}

fn emit_list_runtime(out: &mut String, struct_names: &[String]) {
    let mut emitted_suffixes: HashSet<String> = HashSet::new();
    for (_, c_ty, suffix) in LIST_TYPE_MAP {
        if emitted_suffixes.insert(suffix.to_string()) {
            emit_list_helpers_for(out, c_ty, suffix);
        }
    }
    for struct_name in struct_names {
        if emitted_suffixes.insert(struct_name.clone()) {
            emit_list_helpers_for(out, struct_name, struct_name);
        }
    }
}

fn emit_text_runtime(out: &mut String) {
    out.push_str("static char sk_text_char_at(const char *s, int64_t idx) {\n");
    out.push_str("    if (!s || idx < 0) return '\\0';\n");
    out.push_str("    size_t n = strlen(s);\n");
    out.push_str("    if ((size_t)idx >= n) return '\\0';\n");
    out.push_str("    return s[(size_t)idx];\n");
    out.push_str("}\n\n");
    out.push_str("static int64_t sk_text_find(const char *s, const char *needle) {\n");
    out.push_str("    if (!s || !needle) return -1;\n");
    out.push_str("    const char *p = strstr(s, needle);\n");
    out.push_str("    if (!p) return -1;\n");
    out.push_str("    return (int64_t)(p - s);\n");
    out.push_str("}\n\n");
    out.push_str("static char* sk_text_slice(const char *s, int64_t start, int64_t end) {\n");
    out.push_str("    if (!s) return NULL;\n");
    out.push_str("    int64_t n = (int64_t)strlen(s);\n");
    out.push_str("    if (start < 0) start = 0;\n");
    out.push_str("    if (end < start) end = start;\n");
    out.push_str("    if (start > n) start = n;\n");
    out.push_str("    if (end > n) end = n;\n");
    out.push_str("    size_t len = (size_t)(end - start);\n");
    out.push_str("    char *out = (char*)malloc(len + 1);\n");
    out.push_str("    if (!out) return NULL;\n");
    out.push_str("    if (len > 0) {\n");
    out.push_str("        memcpy(out, s + start, len);\n");
    out.push_str("    }\n");
    out.push_str("    out[len] = '\\0';\n");
    out.push_str("    return (char*)sk_track_alloc(out);\n");
    out.push_str("}\n\n");
    out.push_str("static char* sk_text_concat(const char *a, const char *b) {\n");
    out.push_str("    const char *left = a ? a : \"\";\n");
    out.push_str("    const char *right = b ? b : \"\";\n");
    out.push_str("    size_t alen = strlen(left);\n");
    out.push_str("    size_t blen = strlen(right);\n");
    out.push_str("    char *out = (char*)malloc(alen + blen + 1);\n");
    out.push_str("    if (!out) return sk_strdup_track(\"\");\n");
    out.push_str("    memcpy(out, left, alen);\n");
    out.push_str("    memcpy(out + alen, right, blen);\n");
    out.push_str("    out[alen + blen] = '\\0';\n");
    out.push_str("    return (char*)sk_track_alloc(out);\n");
    out.push_str("}\n\n");
}

fn emit_alloc_runtime(out: &mut String) {
    out.push_str("static void **sk_allocs = NULL;\n");
    out.push_str("static size_t sk_allocs_len = 0;\n");
    out.push_str("static size_t sk_allocs_cap = 0;\n\n");
    out.push_str("static void* sk_track_alloc(void *p) {\n");
    out.push_str("    if (!p) return NULL;\n");
    out.push_str("    if (sk_allocs_len == sk_allocs_cap) {\n");
    out.push_str("        size_t next = sk_allocs_cap == 0 ? 64 : sk_allocs_cap * 2;\n");
    out.push_str("        void **np = (void**)realloc(sk_allocs, next * sizeof(void*));\n");
    out.push_str("        if (!np) return p;\n");
    out.push_str("        sk_allocs = np;\n");
    out.push_str("        sk_allocs_cap = next;\n");
    out.push_str("    }\n");
    out.push_str("    sk_allocs[sk_allocs_len++] = p;\n");
    out.push_str("    return p;\n");
    out.push_str("}\n\n");
    out.push_str("static char* sk_strdup_track(const char *s) {\n");
    out.push_str("    const char *src = s ? s : \"\";\n");
    out.push_str("    size_t n = strlen(src);\n");
    out.push_str("    char *p = (char*)malloc(n + 1);\n");
    out.push_str("    if (!p) return NULL;\n");
    out.push_str("    memcpy(p, src, n + 1);\n");
    out.push_str("    return (char*)sk_track_alloc(p);\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_runtime_cleanup(void) {\n");
    out.push_str("    for (size_t i = 0; i < sk_allocs_len; ++i) {\n");
    out.push_str("        free(sk_allocs[i]);\n");
    out.push_str("    }\n");
    out.push_str("    free(sk_allocs);\n");
    out.push_str("    sk_allocs = NULL;\n");
    out.push_str("    sk_allocs_len = 0;\n");
    out.push_str("    sk_allocs_cap = 0;\n");
    out.push_str("}\n\n");
}

fn emit_fs_runtime(out: &mut String, need_list: bool, need_is_dir: bool, need_join: bool) {
    if need_is_dir {
    out.push_str("static bool sk_fs_is_dir(const char *path) {\n");
    out.push_str("    if (!path) return false;\n");
    out.push_str("    struct stat st;\n");
    out.push_str("    if (stat(path, &st) != 0) return false;\n");
    out.push_str("    return S_ISDIR(st.st_mode) != 0;\n");
    out.push_str("}\n\n");
    }
    if need_list {
    out.push_str("static SkadiList_text sk_fs_list(const char *path) {\n");
    out.push_str("    SkadiList_text out = sk_list_text_new();\n");
    out.push_str("    if (!path) return out;\n");
    out.push_str("    DIR *dir = opendir(path);\n");
    out.push_str("    if (!dir) return out;\n");
    out.push_str("    struct dirent *ent;\n");
    out.push_str("    while ((ent = readdir(dir)) != NULL) {\n");
    out.push_str("        if (strcmp(ent->d_name, \".\") == 0 || strcmp(ent->d_name, \"..\") == 0) continue;\n");
    out.push_str("        char *name = sk_strdup_track(ent->d_name);\n");
    out.push_str("        if (!name) continue;\n");
    out.push_str("        (void)sk_list_text_push(&out, name);\n");
    out.push_str("    }\n");
    out.push_str("    closedir(dir);\n");
    out.push_str("    return out;\n");
    out.push_str("}\n\n");
    }
    if need_join {
        out.push_str("static char* sk_fs_join(const char *a, const char *b) {\n");
        out.push_str("    const char *left = a ? a : \"\";\n");
        out.push_str("    const char *right = b ? b : \"\";\n");
        out.push_str("    size_t alen = strlen(left);\n");
        out.push_str("    size_t blen = strlen(right);\n");
        out.push_str("    bool need_sep = alen > 0 && left[alen - 1] != '/' && left[alen - 1] != '\\\\';\n");
        out.push_str("    size_t n = alen + (need_sep ? 1 : 0) + blen;\n");
        out.push_str("    char *outp = (char*)malloc(n + 1);\n");
        out.push_str("    if (!outp) return sk_strdup_track(\"\");\n");
        out.push_str("    memcpy(outp, left, alen);\n");
        out.push_str("    size_t p = alen;\n");
        out.push_str("    if (need_sep) outp[p++] = '/';\n");
        out.push_str("    memcpy(outp + p, right, blen);\n");
        out.push_str("    outp[n] = '\\0';\n");
        out.push_str("    return (char*)sk_track_alloc(outp);\n");
        out.push_str("}\n\n");
    }
}

fn emit_io_runtime(out: &mut String, needs_args_runtime: bool) {
    out.push_str("static int sk_output_text(const char *s) { printf(\"%s\\n\", s ? s : \"\"); return 0; }\n");
    out.push_str("static int sk_output_int(int64_t v) { printf(\"%lld\\n\", (long long)v); return 0; }\n");
    out.push_str("static int sk_output_float(double v) { printf(\"%f\\n\", v); return 0; }\n");
    out.push_str("static int sk_output_bool(bool v) { printf(\"%s\\n\", v ? \"true\" : \"false\"); return 0; }\n");
    out.push_str("static int sk_output_char(char v) { printf(\"%c\\n\", v); return 0; }\n\n");
    out.push_str("static char* sk_input(const char *prompt) {\n");
    out.push_str("    if (prompt) printf(\"%s\", prompt);\n");
    out.push_str("    char buf[4096];\n");
    out.push_str("    if (!fgets(buf, sizeof(buf), stdin)) return sk_strdup_track(\"\");\n");
    out.push_str("    size_t n = strlen(buf);\n");
    out.push_str("    if (n > 0 && buf[n - 1] == '\\n') buf[n - 1] = '\\0';\n");
    out.push_str("    return sk_strdup_track(buf);\n");
    out.push_str("}\n\n");
    out.push_str("static char* sk_read_file(const char *path) {\n");
    out.push_str("    FILE *f = fopen(path, \"rb\");\n");
    out.push_str("    if (!f) return sk_strdup_track(\"\");\n");
    out.push_str("    fseek(f, 0, SEEK_END);\n");
    out.push_str("    long n = ftell(f);\n");
    out.push_str("    fseek(f, 0, SEEK_SET);\n");
    out.push_str("    if (n < 0) { fclose(f); return sk_strdup_track(\"\"); }\n");
    out.push_str("    char *buf = (char*)malloc((size_t)n + 1);\n");
    out.push_str("    if (!buf) { fclose(f); return sk_strdup_track(\"\"); }\n");
    out.push_str("    size_t r = fread(buf, 1, (size_t)n, f);\n");
    out.push_str("    buf[r] = '\\0';\n");
    out.push_str("    fclose(f);\n");
    out.push_str("    return (char*)sk_track_alloc(buf);\n");
    out.push_str("}\n\n");
    out.push_str("static int sk_write_file(const char *path, const char *data) {\n");
    out.push_str("    FILE *f = fopen(path, \"wb\");\n");
    out.push_str("    if (!f) return 1;\n");
    out.push_str("    size_t n = data ? strlen(data) : 0;\n");
    out.push_str("    size_t w = fwrite(data ? data : \"\", 1, n, f);\n");
    out.push_str("    fclose(f);\n");
    out.push_str("    return w == n ? 0 : 1;\n");
    out.push_str("}\n\n");
    if needs_args_runtime {
        out.push_str("static SkadiList_text sk_args(int argc, char **argv) {\n");
        out.push_str("    SkadiList_text out = sk_list_text_new();\n");
        out.push_str("    for (int i = 1; i < argc; ++i) {\n");
        out.push_str("        char *v = sk_strdup_track(argv[i] ? argv[i] : \"\");\n");
        out.push_str("        if (!v) continue;\n");
        out.push_str("        (void)sk_list_text_push(&out, v);\n");
        out.push_str("    }\n");
        out.push_str("    return out;\n");
        out.push_str("}\n\n");
    }
}

pub fn transpile_program_to_c(program: &Program) -> String {
    let mut out = String::new();
    let struct_names = collect_struct_names(program);
    let (needs_fs_list, needs_fs_is_dir, needs_fs_join) = program_uses_fs_runtime(program);
    let needs_list_runtime = program_uses_list_runtime(program) || needs_fs_list;
    let needs_text_runtime = program_uses_text_runtime(program);
    let needs_io_runtime = program_uses_io_runtime(program);
    let needs_args_runtime = program_uses_args_runtime(program);
    let needs_alloc_runtime =
        needs_text_runtime || needs_fs_list || needs_fs_join || needs_io_runtime || needs_args_runtime;
    out.push_str("#include <stdio.h>\n\n");
    if needs_list_runtime || needs_text_runtime || needs_io_runtime {
        out.push_str("#include <stddef.h>\n");
        out.push_str("#include <stdlib.h>\n");
    }
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stdbool.h>\n\n");
    if needs_text_runtime || needs_fs_list || needs_fs_join || needs_io_runtime || needs_args_runtime {
        out.push_str("#include <string.h>\n\n");
    }
    if needs_fs_list || needs_fs_is_dir || needs_fs_join {
        out.push_str("#include <dirent.h>\n");
        out.push_str("#include <sys/stat.h>\n\n");
    }
    emit_struct_declarations(program, &mut out);
    if needs_alloc_runtime {
        emit_alloc_runtime(&mut out);
    }
    if needs_list_runtime {
        emit_list_runtime(&mut out, &struct_names);
    }
    if needs_text_runtime {
        emit_text_runtime(&mut out);
    }
    if needs_fs_list || needs_fs_is_dir || needs_fs_join {
        emit_fs_runtime(&mut out, needs_fs_list, needs_fs_is_dir, needs_fs_join);
    }
    if needs_io_runtime {
        emit_io_runtime(&mut out, needs_args_runtime);
    }
    emit_error_code_enum(program, &mut out);

    for stmt in &program.statements {
        if let Statement::FunctionDef { .. } = stmt {
            emit_function(stmt, &mut out);
            out.push('\n');
        }
    }
    emit_struct_methods(program, &mut out);

    if needs_args_runtime {
        out.push_str("int main(int argc, char **argv) {\n");
    } else {
        out.push_str("int main(void) {\n");
    }
    let mut declared: HashMap<String, String> = HashMap::new();
    for stmt in &program.statements {
        if !matches!(stmt, Statement::FunctionDef { .. }) {
            emit_statement(stmt, &mut out, 1, &mut declared, None);
        }
    }
    for (name, ty) in &declared {
        if let Some(elem) = list_elem_from_decl(ty) {
            let suffix = list_meta_dynamic(elem).1;
            out.push_str("    sk_list_");
            out.push_str(&suffix);
            out.push_str("_free(&");
            out.push_str(name);
            out.push_str(");\n");
        }
    }
    if needs_alloc_runtime {
        out.push_str("    sk_runtime_cleanup();\n");
    }
    out.push_str("    return 0;\n");
    out.push_str("}\n");

    out
}

fn emit_struct_declarations(program: &Program, out: &mut String) {
    for stmt in &program.statements {
        if let Statement::StructDecl { name, fields, .. } = stmt {
            out.push_str("typedef struct {\n");
            for field in fields {
                let c_ty = map_skadi_type_to_c(Some(field.field_type.as_str()));
                out.push_str("    ");
                out.push_str(&c_ty);
                out.push(' ');
                out.push_str(&field.name);
                out.push_str(";\n");
            }
            out.push_str("} ");
            out.push_str(name);
            out.push_str(";\n\n");
        }
    }
}

fn emit_struct_methods(program: &Program, out: &mut String) {
    for stmt in &program.statements {
        let Statement::StructDecl { name, methods, .. } = stmt else { continue };
        for method in methods {
            if method.is_danger {
                out.push_str("int ");
            } else {
                out.push_str(&map_skadi_type_to_c(method.returns.as_deref()));
                out.push(' ');
            }
            out.push_str(name);
            out.push('_');
            out.push_str(&method.name);
            out.push('(');
            out.push_str(name);
            out.push_str(" *my");
            for p in &method.params {
                out.push_str(", ");
                out.push_str(&map_skadi_type_to_c(p.param_type.as_deref()));
                out.push(' ');
                out.push_str(&p.name);
            }
            if method.is_danger && let Some(ret_ty) = method.returns.as_deref() {
                out.push_str(", ");
                out.push_str(&map_skadi_type_to_c(Some(ret_ty)));
                out.push_str(" *out");
            }
            out.push_str(") {\n");
            let mut declared = HashMap::new();
            declared.insert("my".to_string(), name.clone());
            for p in &method.params {
                declared.insert(
                    p.name.clone(),
                    p.param_type.clone().unwrap_or_else(|| "Int".to_string()),
                );
            }
            let fn_ctx = FunctionContext {
                is_danger: method.is_danger,
                return_type: method.returns.clone(),
            };
            emit_block(&method.body, out, 1, &mut declared, Some(&fn_ctx));
            if method.is_danger {
                out.push_str("    return 1;\n");
            } else if method.returns.as_deref().is_some() {
                out.push_str("    return 0;\n");
            }
            out.push_str("}\n\n");
        }
    }
}

fn program_uses_text_runtime(program: &Program) -> bool {
    fn expression_needs_text_runtime(expr: &Expression) -> bool {
        match expr {
            Expression::LiteralString(_) => true,
            Expression::ListLiteral(items) => items.iter().any(expression_needs_text_runtime),
            Expression::Index { base, index } => {
                expression_needs_text_runtime(base) || expression_needs_text_runtime(index)
            }
            Expression::Call { name, args } => {
                matches!(
                    name.as_str(),
                    "len" | "contains" | "find" | "slice" | "concat" | "input" | "read" | "fs.join"
                ) || args.iter().any(expression_needs_text_runtime)
            }
            Expression::BinaryOp { left, right, .. } => {
                expression_needs_text_runtime(left)
                    || right
                        .as_ref()
                        .map(|r| expression_needs_text_runtime(r))
                        .unwrap_or(false)
            }
            Expression::StructConstruction { fields } => {
                fields.values().any(|v| expression_needs_text_runtime(v))
            }
            _ => false,
        }
    }

    fn block_has_text(block: &BlockStatement) -> bool {
        block.statements.iter().any(statement_has_text)
    }

    fn statement_has_text(stmt: &Statement) -> bool {
        match stmt {
            Statement::VarDecl {
                declared_type, value, ..
            } => {
                declared_type
                    .as_deref()
                    .map(|t| t == "Text")
                    .unwrap_or(false)
                    || expression_needs_text_runtime(value)
            }
            Statement::Assignment { value, .. }
            | Statement::FieldAssignment { value, .. }
            | Statement::ExpressionStatement { expr: value, .. } => expression_needs_text_runtime(value),
            Statement::FunctionDef { body, .. } => block_has_text(body),
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                expression_needs_text_runtime(condition)
                    || block_has_text(then_block)
                    || else_block
                        .as_ref()
                        .map(|b| block_has_text(b))
                        .unwrap_or(false)
            }
            Statement::ForLoop {
                initialization,
                condition,
                update,
                body,
                ..
            } => {
                initialization
                    .as_ref()
                    .map(|e| expression_needs_text_runtime(e))
                    .unwrap_or(false)
                    || condition
                        .as_ref()
                        .map(|e| expression_needs_text_runtime(e))
                        .unwrap_or(false)
                    || update
                        .as_ref()
                        .map(|e| expression_needs_text_runtime(e))
                        .unwrap_or(false)
                    || block_has_text(body)
            }
            Statement::WhenBlock {
                when_expression,
                cases,
                else_block,
                ..
            } => {
                expression_needs_text_runtime(when_expression)
                    || cases.iter().any(|(case_exprs, b)| {
                        case_exprs.iter().any(expression_needs_text_runtime) || block_has_text(b)
                    })
                    || else_block
                        .as_ref()
                        .map(|b| block_has_text(b))
                        .unwrap_or(false)
            }
            Statement::WhileLoop {
                condition, body, ..
            } => expression_needs_text_runtime(condition) || block_has_text(body),
            Statement::LoopStatement { body, .. } => block_has_text(body),
            Statement::DangerAssignOnError { args, on_error, .. }
            | Statement::DangerCallOnError { args, on_error, .. } => {
                args.iter().any(expression_needs_text_runtime) || block_has_text(on_error)
            }
            Statement::ListPush { value, .. } => expression_needs_text_runtime(value),
            Statement::ListPopOnError { on_error, .. } => block_has_text(on_error),
            Statement::ReturnStatement { value, .. } => value
                .as_ref()
                .map(|v| expression_needs_text_runtime(v))
                .unwrap_or(false),
            Statement::BlockStatement { statements, .. } | Statement::OnErrorBlock { statements, .. } => {
                statements.iter().any(statement_has_text)
            }
            _ => false,
        }
    }
    program.statements.iter().any(statement_has_text)
}

fn program_uses_list_runtime(program: &Program) -> bool {
    fn block_has_for(block: &BlockStatement) -> bool {
        block.statements.iter().any(statement_needs_list)
    }
    fn statement_needs_list(stmt: &Statement) -> bool {
        match stmt {
            Statement::ForLoop { .. } => true,
            Statement::VarDecl { declared_type, .. } => declared_type
                .as_deref()
                .map(|t| t.ends_with(" List"))
                .unwrap_or(false),
            Statement::ListPush { .. } | Statement::ListPopOnError { .. } => true,
            Statement::FunctionDef { body, .. } => block_has_for(body),
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                block_has_for(then_block)
                    || else_block
                        .as_ref()
                        .map(|b| block_has_for(b))
                        .unwrap_or(false)
            }
            Statement::WhenBlock { cases, else_block, .. } => {
                cases.iter().any(|(_, b)| block_has_for(b))
                    || else_block
                        .as_ref()
                        .map(|b| block_has_for(b))
                        .unwrap_or(false)
            }
            Statement::WhileLoop { body, .. } | Statement::LoopStatement { body, .. } => block_has_for(body),
            Statement::DangerAssignOnError { on_error, .. }
            | Statement::DangerCallOnError { on_error, .. } => block_has_for(on_error),
            Statement::BlockStatement { statements, .. } | Statement::OnErrorBlock { statements, .. } => {
                statements.iter().any(statement_needs_list)
            }
            _ => false,
        }
    }
    program.statements.iter().any(statement_needs_list)
}

fn expression_uses_fs_call(expr: &Expression) -> (bool, bool, bool) {
    match expr {
        Expression::Call { name, args } => {
            let mut needs_list = name == "fs.list";
            let mut needs_is_dir = name == "fs.is_dir";
            let mut needs_join = name == "fs.join";
            for a in args {
                let (l, d, j) = expression_uses_fs_call(a);
                needs_list |= l;
                needs_is_dir |= d;
                needs_join |= j;
            }
            (needs_list, needs_is_dir, needs_join)
        }
        Expression::BinaryOp { left, right, .. } => {
            let (mut l1, mut d1, mut j1) = expression_uses_fs_call(left);
            if let Some(r) = right {
                let (l2, d2, j2) = expression_uses_fs_call(r);
                l1 |= l2;
                d1 |= d2;
                j1 |= j2;
            }
            (l1, d1, j1)
        }
        Expression::Index { base, index } => {
            let (l1, d1, j1) = expression_uses_fs_call(base);
            let (l2, d2, j2) = expression_uses_fs_call(index);
            (l1 || l2, d1 || d2, j1 || j2)
        }
        Expression::ListLiteral(items) => {
            let mut nl = false;
            let mut nd = false;
            let mut nj = false;
            for it in items {
                let (l, d, j) = expression_uses_fs_call(it);
                nl |= l;
                nd |= d;
                nj |= j;
            }
            (nl, nd, nj)
        }
        Expression::StructConstruction { fields } => {
            let mut nl = false;
            let mut nd = false;
            let mut nj = false;
            for v in fields.values() {
                let (l, d, j) = expression_uses_fs_call(v);
                nl |= l;
                nd |= d;
                nj |= j;
            }
            (nl, nd, nj)
        }
        _ => (false, false, false),
    }
}

fn program_uses_fs_runtime(program: &Program) -> (bool, bool, bool) {
    fn statements_uses_fs(statements: &[Statement]) -> (bool, bool, bool) {
        let mut nl = false;
        let mut nd = false;
        let mut nj = false;
        for s in statements {
            let (l, d, j) = stmt_uses_fs(s);
            nl |= l;
            nd |= d;
            nj |= j;
        }
        (nl, nd, nj)
    }
    fn block_uses_fs(block: &BlockStatement) -> (bool, bool, bool) {
        statements_uses_fs(&block.statements)
    }
    fn stmt_uses_fs(stmt: &Statement) -> (bool, bool, bool) {
        match stmt {
            Statement::VarDecl { value, .. } => expression_uses_fs_call(value),
            Statement::Assignment { value, .. } => expression_uses_fs_call(value),
            Statement::FunctionDef { body, .. } => block_uses_fs(body),
            Statement::IfStatement { condition, then_block, else_block, .. } => {
                let (mut nl, mut nd, mut nj) = expression_uses_fs_call(condition);
                let (l2, d2, j2) = block_uses_fs(then_block);
                nl |= l2;
                nd |= d2;
                nj |= j2;
                if let Some(b) = else_block {
                    let (l3, d3, j3) = block_uses_fs(b);
                    nl |= l3;
                    nd |= d3;
                    nj |= j3;
                }
                (nl, nd, nj)
            }
            Statement::ForLoop { condition, body, .. } => {
                let (mut nl, mut nd, mut nj) = condition
                    .as_ref()
                    .map(|e| expression_uses_fs_call(e))
                    .unwrap_or((false, false, false));
                let (l2, d2, j2) = block_uses_fs(body);
                nl |= l2;
                nd |= d2;
                nj |= j2;
                (nl, nd, nj)
            }
            Statement::WhenBlock { when_expression, cases, else_block, .. } => {
                let (mut nl, mut nd, mut nj) = expression_uses_fs_call(when_expression);
                for (_, b) in cases {
                    let (l, d, j) = block_uses_fs(b);
                    nl |= l;
                    nd |= d;
                    nj |= j;
                }
                if let Some(b) = else_block {
                    let (l, d, j) = block_uses_fs(b);
                    nl |= l;
                    nd |= d;
                    nj |= j;
                }
                (nl, nd, nj)
            }
            Statement::WhileLoop { condition, body, .. } => {
                let (mut nl, mut nd, mut nj) = expression_uses_fs_call(condition);
                let (l2, d2, j2) = block_uses_fs(body);
                nl |= l2;
                nd |= d2;
                nj |= j2;
                (nl, nd, nj)
            }
            Statement::LoopStatement { body, .. } => block_uses_fs(body),
            Statement::DangerAssignOnError { args, on_error, .. }
            | Statement::DangerCallOnError { args, on_error, .. } => {
                let mut nl = false;
                let mut nd = false;
                let mut nj = false;
                for a in args {
                    let (l, d, j) = expression_uses_fs_call(a);
                    nl |= l;
                    nd |= d;
                    nj |= j;
                }
                let (l2, d2, j2) = block_uses_fs(on_error);
                (nl || l2, nd || d2, nj || j2)
            }
            Statement::ListPush { value, .. } => expression_uses_fs_call(value),
            Statement::ListPopOnError { on_error, .. }
            => block_uses_fs(on_error),
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => statements_uses_fs(statements),
            Statement::ReturnStatement { value, .. } => value
                .as_ref()
                .map(|v| expression_uses_fs_call(v))
                .unwrap_or((false, false, false)),
            _ => (false, false, false),
        }
    }

    let mut nl = false;
    let mut nd = false;
    let mut nj = false;
    for s in &program.statements {
        let (l, d, j) = stmt_uses_fs(s);
        nl |= l;
        nd |= d;
        nj |= j;
    }
    (nl, nd, nj)
}

fn expression_uses_io_call(expr: &Expression) -> bool {
    match expr {
        Expression::Call { name, args } => {
            let is_io = matches!(name.as_str(), "output" | "input" | "read" | "write" | "args");
            is_io || args.iter().any(expression_uses_io_call)
        }
        Expression::BinaryOp { left, right, .. } => {
            expression_uses_io_call(left)
                || right.as_deref().map(expression_uses_io_call).unwrap_or(false)
        }
        Expression::Index { base, index } => {
            expression_uses_io_call(base) || expression_uses_io_call(index)
        }
        Expression::ListLiteral(items) => items.iter().any(expression_uses_io_call),
        Expression::StructConstruction { fields } => fields.values().any(|v| expression_uses_io_call(v)),
        _ => false,
    }
}

fn expression_uses_args_call(expr: &Expression) -> bool {
    match expr {
        Expression::Call { name, args } => {
            name == "args" || args.iter().any(expression_uses_args_call)
        }
        Expression::BinaryOp { left, right, .. } => {
            expression_uses_args_call(left)
                || right.as_deref().map(expression_uses_args_call).unwrap_or(false)
        }
        Expression::Index { base, index } => {
            expression_uses_args_call(base) || expression_uses_args_call(index)
        }
        Expression::ListLiteral(items) => items.iter().any(expression_uses_args_call),
        Expression::StructConstruction { fields } => fields.values().any(|v| expression_uses_args_call(v)),
        _ => false,
    }
}

fn program_uses_io_runtime(program: &Program) -> bool {
    fn stmt_uses_io(stmt: &Statement) -> bool {
        match stmt {
            Statement::VarDecl { value, .. } => expression_uses_io_call(value),
            Statement::Assignment { value, .. } => expression_uses_io_call(value),
            Statement::FunctionDef { body, .. } => body.statements.iter().any(stmt_uses_io),
            Statement::ExpressionStatement { expr, .. } => expression_uses_io_call(expr),
            Statement::IfStatement { condition, then_block, else_block, .. } => {
                expression_uses_io_call(condition)
                    || then_block.statements.iter().any(stmt_uses_io)
                    || else_block
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_io))
                        .unwrap_or(false)
            }
            Statement::ForLoop { condition, body, .. } => {
                condition.as_ref().map(|e| expression_uses_io_call(e)).unwrap_or(false)
                    || body.statements.iter().any(stmt_uses_io)
            }
            Statement::WhenBlock { when_expression, cases, else_block, .. } => {
                expression_uses_io_call(when_expression)
                    || cases.iter().any(|(_, b)| b.statements.iter().any(stmt_uses_io))
                    || else_block
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_io))
                        .unwrap_or(false)
            }
            Statement::WhileLoop { condition, body, .. } => {
                expression_uses_io_call(condition) || body.statements.iter().any(stmt_uses_io)
            }
            Statement::LoopStatement { body, .. } => body.statements.iter().any(stmt_uses_io),
            Statement::DangerAssignOnError { args, on_error, .. }
            | Statement::DangerCallOnError { args, on_error, .. } => {
                args.iter().any(expression_uses_io_call) || on_error.statements.iter().any(stmt_uses_io)
            }
            Statement::ListPush { value, .. } => expression_uses_io_call(value),
            Statement::ListPopOnError { on_error, .. } => on_error.statements.iter().any(stmt_uses_io),
            Statement::ReturnStatement { value, .. } => {
                value.as_ref().map(|v| expression_uses_io_call(v)).unwrap_or(false)
            }
            Statement::BlockStatement { statements, .. } | Statement::OnErrorBlock { statements, .. } => {
                statements.iter().any(stmt_uses_io)
            }
            _ => false,
        }
    }
    program.statements.iter().any(stmt_uses_io)
}

fn program_uses_args_runtime(program: &Program) -> bool {
    fn stmt_uses_args(stmt: &Statement) -> bool {
        match stmt {
            Statement::VarDecl { value, .. } => expression_uses_args_call(value),
            Statement::Assignment { value, .. } => expression_uses_args_call(value),
            Statement::FunctionDef { body, .. } => body.statements.iter().any(stmt_uses_args),
            Statement::ExpressionStatement { expr, .. } => expression_uses_args_call(expr),
            Statement::IfStatement { condition, then_block, else_block, .. } => {
                expression_uses_args_call(condition)
                    || then_block.statements.iter().any(stmt_uses_args)
                    || else_block
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_args))
                        .unwrap_or(false)
            }
            Statement::ForLoop { condition, body, .. } => {
                condition.as_ref().map(|e| expression_uses_args_call(e)).unwrap_or(false)
                    || body.statements.iter().any(stmt_uses_args)
            }
            Statement::WhenBlock { when_expression, cases, else_block, .. } => {
                expression_uses_args_call(when_expression)
                    || cases.iter().any(|(_, b)| b.statements.iter().any(stmt_uses_args))
                    || else_block
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_args))
                        .unwrap_or(false)
            }
            Statement::WhileLoop { condition, body, .. } => {
                expression_uses_args_call(condition) || body.statements.iter().any(stmt_uses_args)
            }
            Statement::LoopStatement { body, .. } => body.statements.iter().any(stmt_uses_args),
            Statement::DangerAssignOnError { args, on_error, .. }
            | Statement::DangerCallOnError { args, on_error, .. } => {
                args.iter().any(expression_uses_args_call) || on_error.statements.iter().any(stmt_uses_args)
            }
            Statement::ListPush { value, .. } => expression_uses_args_call(value),
            Statement::ListPopOnError { on_error, .. } => on_error.statements.iter().any(stmt_uses_args),
            Statement::ReturnStatement { value, .. } => {
                value.as_ref().map(|v| expression_uses_args_call(v)).unwrap_or(false)
            }
            Statement::BlockStatement { statements, .. } | Statement::OnErrorBlock { statements, .. } => {
                statements.iter().any(stmt_uses_args)
            }
            _ => false,
        }
    }
    program.statements.iter().any(stmt_uses_args)
}

fn emit_error_code_enum(program: &Program, out: &mut String) {
    for stmt in &program.statements {
        if let Statement::LabelDecl { name, variants, .. } = stmt
            && name == "ErrorCode"
            && !variants.is_empty()
        {
            out.push_str("typedef enum ErrorCode {\n");
            for (i, v) in variants.iter().enumerate() {
                if i == 0 {
                    out.push_str(&format!("    ErrorCode_{} = 0,\n", v));
                } else {
                    out.push_str(&format!("    ErrorCode_{} = {},\n", v, i));
                }
            }
            out.push_str("} ErrorCode;\n\n");
            break;
        }
    }
}

fn emit_function(stmt: &Statement, out: &mut String) {
    if let Statement::FunctionDef {
        name,
        params,
        body,
        returns,
        is_danger,
        ..
    } = stmt
    {
        if *is_danger {
            out.push_str("int");
        } else {
            out.push_str(&map_skadi_type_to_c(returns.as_deref()));
        }
        out.push(' ');
        out.push_str(name);
        out.push('(');
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&map_skadi_type_to_c(p.param_type.as_deref()));
            out.push(' ');
            out.push_str(&p.name);
        }
        if *is_danger && let Some(ret_ty) = returns.as_deref() {
            if !params.is_empty() {
                out.push_str(", ");
            }
            out.push_str(&map_skadi_type_to_c(Some(ret_ty)));
            out.push_str(" *out");
        }
        out.push_str(") {\n");
        let mut declared: HashMap<String, String> = params
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    p.param_type.clone().unwrap_or_else(|| "Int".to_string()),
                )
            })
            .collect();
        let fn_ctx = FunctionContext {
            is_danger: *is_danger,
            return_type: returns.clone(),
        };
        emit_block(body, out, 1, &mut declared, Some(&fn_ctx));
        out.push_str("    return 0;\n");
        out.push_str("}\n");
    }
}

fn emit_block(
    block: &BlockStatement,
    out: &mut String,
    indent: usize,
    declared: &mut HashMap<String, String>,
    fn_ctx: Option<&FunctionContext>,
) {
    for stmt in &block.statements {
        emit_statement(stmt, out, indent, declared, fn_ctx);
    }
}

fn emit_statement(
    stmt: &Statement,
    out: &mut String,
    indent: usize,
    declared: &mut HashMap<String, String>,
    fn_ctx: Option<&FunctionContext>,
) {
    let pad = "    ".repeat(indent);
    match stmt {
        Statement::Assignment { target, value, .. } => {
            let expr = emit_expr(value, declared);
            out.push_str(&pad);
            out.push_str(target);
            out.push_str(" = ");
            out.push_str(&expr);
            out.push_str(";\n");
        }
        Statement::FieldAssignment {
            object, field, value, ..
        } => {
            out.push_str(&pad);
            if object == "my" {
                out.push_str("my->");
            } else {
                out.push_str(object);
                out.push('.');
            }
            out.push_str(field);
            out.push_str(" = ");
            out.push_str(&emit_expr(value, declared));
            out.push_str(";\n");
        }
        Statement::IfStatement { condition, then_block, else_block, .. } => {
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(&emit_expr(condition, declared));
            out.push_str(") {\n");
            let mut then_decl = declared.clone();
            emit_block(then_block, out, indent + 1, &mut then_decl, fn_ctx);
            out.push_str(&pad);
            out.push('}');
            if let Some(else_block) = else_block {
                out.push_str(" else {\n");
                let mut else_decl = declared.clone();
                emit_block(else_block, out, indent + 1, &mut else_decl, fn_ctx);
                out.push_str(&pad);
                out.push('}');
            }
            out.push('\n');
        }
        Statement::WhileLoop { condition, body, .. } => {
            out.push_str(&pad);
            out.push_str("while (");
            out.push_str(&emit_expr(condition, declared));
            out.push_str(") {\n");
            let mut inner = declared.clone();
            emit_block(body, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::LoopStatement { body, .. } => {
            out.push_str(&pad);
            out.push_str("while (1) {\n");
            let mut inner = declared.clone();
            emit_block(body, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::ForLoop { initialization, condition, update, body, .. } => {
            if let (Some(init), Some(coll)) = (initialization, condition) {
                let var_name = match init.as_ref() {
                    Expression::VariableReference(v) => v.clone(),
                    _ => "item".to_string(),
                };
                let coll_expr = emit_expr(coll, declared);
                let item_c_ty = match coll.as_ref() {
                    Expression::VariableReference(coll_name) => declared
                        .get(coll_name)
                        .and_then(|t| list_elem_from_decl(t))
                        .map(|elem| list_meta_dynamic(elem).0)
                        .unwrap_or_else(|| "int64_t".to_string()),
                    _ => "int64_t".to_string(),
                };
                let item_decl_ty = match coll.as_ref() {
                    Expression::VariableReference(coll_name) => declared
                        .get(coll_name)
                        .and_then(|t| list_elem_from_decl(t))
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Int".to_string()),
                    _ => "Int".to_string(),
                };
                out.push_str(&pad);
                out.push_str("for (size_t __i = 0; __i < ");
                out.push_str(&coll_expr);
                out.push_str(".len; ++__i) {\n");
                out.push_str(&"    ".repeat(indent + 1));
                out.push_str(&item_c_ty);
                out.push(' ');
                out.push_str(&var_name);
                out.push_str(" = ");
                out.push_str(&coll_expr);
                out.push_str(".data[__i]");
                out.push_str(";\n");
                let mut inner = declared.clone();
                inner.insert(var_name, item_decl_ty);
                emit_block(body, out, indent + 1, &mut inner, fn_ctx);
                out.push_str(&pad);
                out.push_str("}\n");
            } else {
                out.push_str(&pad);
                out.push_str("for (");
                if let Some(init) = initialization {
                    out.push_str(&emit_expr(init, declared));
                }
                out.push_str("; ");
                if let Some(cond) = condition {
                    out.push_str(&emit_expr(cond, declared));
                }
                out.push_str("; ");
                if let Some(step) = update {
                    out.push_str(&emit_expr(step, declared));
                }
                out.push_str(") {\n");
                let mut inner = declared.clone();
                emit_block(body, out, indent + 1, &mut inner, fn_ctx);
                out.push_str(&pad);
                out.push_str("}\n");
            }
        }
        Statement::FunctionDef { .. } => {}
        Statement::LabelDecl { name, .. } => {
            out.push_str(&pad);
            out.push_str("/* label ");
            out.push_str(name);
            out.push_str(" */\n");
        }
        Statement::StructDecl { name, .. } => {
            out.push_str(&pad);
            out.push_str("/* struct ");
            out.push_str(name);
            out.push_str(" lowered as typedef above */\n");
        }
        Statement::OnBlock { trigger, .. } => {
            out.push_str(&pad);
            out.push_str("/* on ");
            out.push_str(trigger);
            out.push_str(" TODO(v1): runtime binding */\n");
        }
        Statement::BreakStatement { .. } => {
            out.push_str(&pad);
            out.push_str("break;\n");
        }
        Statement::ContinueStatement { .. } => {
            out.push_str(&pad);
            out.push_str("continue;\n");
        }
        Statement::PassStatement { .. } => {
            out.push_str(&pad);
            out.push_str("/* pass */\n");
        }
        Statement::IncrementStatement { target, .. } => {
            out.push_str(&pad);
            out.push_str(target);
            out.push_str(" += 1;\n");
        }
        Statement::DecrementStatement { target, .. } => {
            out.push_str(&pad);
            out.push_str(target);
            out.push_str(" -= 1;\n");
        }
        Statement::DangerAssignOnError {
            target,
            call_name,
            args,
            on_error,
            ..
        } => {
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(call_name);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&emit_expr(a, declared));
            }
            if !args.is_empty() {
                out.push_str(", ");
            }
            out.push('&');
            out.push_str(target);
            out.push_str(") != 0) {\n");
            let mut inner = declared.clone();
            emit_block(on_error, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::DangerCallOnError {
            call_name,
            args,
            on_error,
            ..
        } => {
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(call_name);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&emit_expr(a, declared));
            }
            out.push_str(") != 0) {\n");
            let mut inner = declared.clone();
            emit_block(on_error, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::ListPush { list_name, value, .. } => {
            let elem_type = declared
                .get(list_name)
                .and_then(|t| list_elem_from_decl(t))
                .map(str::to_string);
            let suffix = declared
                .get(list_name)
                .and_then(|t| list_elem_from_decl(t))
                .map(|elem| list_meta_dynamic(elem).1)
                .unwrap_or_else(|| "i64".to_string());
            let rendered_value = match (&elem_type, value.as_ref()) {
                (Some(elem), Expression::StructConstruction { fields }) => {
                    emit_struct_literal(fields, Some(elem.as_str()), declared)
                }
                _ => emit_expr(value, declared),
            };
            out.push_str(&pad);
            out.push_str("(void)sk_list_");
            out.push_str(&suffix);
            out.push_str("_push(&");
            out.push_str(list_name);
            out.push_str(", ");
            out.push_str(&rendered_value);
            out.push_str(");\n");
        }
        Statement::ListPopOnError {
            target,
            list_name,
            on_error,
            ..
        } => {
            let suffix = declared
                .get(list_name)
                .and_then(|t| list_elem_from_decl(t))
                .map(|elem| list_meta_dynamic(elem).1)
                .unwrap_or_else(|| "i64".to_string());
            out.push_str(&pad);
            out.push_str("if (sk_list_");
            out.push_str(&suffix);
            out.push_str("_pop(&");
            out.push_str(list_name);
            out.push_str(", &");
            out.push_str(target);
            out.push_str(") != 0) {\n");
            let mut inner = declared.clone();
            emit_block(on_error, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::ReturnStatement { value, .. } => {
            if let Some(ctx) = fn_ctx
                && ctx.is_danger
            {
                match (ctx.return_type.is_some(), value) {
                    (true, Some(expr)) => {
                        out.push_str(&pad);
                        out.push_str("*out = ");
                        out.push_str(&emit_expr(expr, declared));
                        out.push_str(";\n");
                        out.push_str(&pad);
                        out.push_str("return 0;\n");
                        return;
                    }
                    (true, None) => {
                        out.push_str(&pad);
                        out.push_str("return 1;\n");
                        return;
                    }
                    (false, Some(expr)) => {
                        out.push_str(&pad);
                        out.push_str("return ");
                        out.push_str(&emit_expr(expr, declared));
                        out.push_str(";\n");
                        return;
                    }
                    (false, None) => {
                        out.push_str(&pad);
                        out.push_str("return 1;\n");
                        return;
                    }
                }
            }
            out.push_str(&pad);
            out.push_str("return");
            if let Some(expr) = value {
                out.push(' ');
                out.push_str(&emit_expr(expr, declared));
            }
            out.push_str(";\n");
        }
        Statement::ExpressionStatement { expr, .. } => {
            out.push_str(&pad);
            out.push_str(&emit_expr(expr, declared));
            out.push_str(";\n");
        }
        Statement::ReturnError { code, .. } => {
            out.push_str(&pad);
            out.push_str("return ErrorCode_");
            out.push_str(code);
            out.push_str(";\n");
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            if cases.is_empty() {
                if let Some(else_block) = else_block {
                    emit_block(else_block, out, indent, declared, fn_ctx);
                }
                return;
            }
            let when_expr = emit_expr(when_expression, declared);
            let when_tmp = format!("__when_tmp_{}", indent);
            let when_is_text = is_text_expr(when_expression, declared)
                || cases.iter().any(|(case_exprs, _)| case_exprs.iter().any(|e| is_text_expr(e, declared)));
            out.push_str(&pad);
            if when_is_text {
                out.push_str("const char* ");
            } else {
                out.push_str("int64_t ");
            }
            out.push_str(&when_tmp);
            out.push_str(" = ");
            out.push_str(&when_expr);
            out.push_str(";\n");
            for (idx, (case_exprs, case_block)) in cases.iter().enumerate() {
                out.push_str(&pad);
                if idx == 0 {
                    out.push_str("if (");
                } else {
                    out.push_str("else if (");
                }
                if case_exprs.is_empty() {
                    out.push('0');
                } else {
                    for (j, expr) in case_exprs.iter().enumerate() {
                        if j > 0 {
                            out.push_str(" || ");
                        }
                        if when_is_text {
                            out.push_str("(strcmp(");
                            out.push_str(&when_tmp);
                            out.push_str(", ");
                            out.push_str(&emit_expr(expr, declared));
                            out.push_str(") == 0)");
                        } else {
                            out.push('(');
                            out.push_str(&when_tmp);
                            out.push_str(" == ");
                            out.push_str(&emit_expr(expr, declared));
                            out.push(')');
                        }
                    }
                }
                out.push_str(") {\n");
                let mut inner = declared.clone();
                emit_block(case_block, out, indent + 1, &mut inner, fn_ctx);
                out.push_str(&pad);
                out.push_str("}\n");
            }
            if let Some(else_block) = else_block {
                out.push_str(&pad);
                out.push_str("else {\n");
                let mut inner = declared.clone();
                emit_block(else_block, out, indent + 1, &mut inner, fn_ctx);
                out.push_str(&pad);
                out.push_str("}\n");
            }
        }
        Statement::VarDecl { name, value, declared_type, .. } => {
            if let Some(dt) = declared_type.as_deref()
                && let Some(elem) = list_elem_from_decl(dt)
            {
                let suffix = list_meta_dynamic(elem).1;
                out.push_str(&pad);
                out.push_str("SkadiList_");
                out.push_str(&suffix);
                out.push(' ');
                out.push_str(name);
                out.push_str(" = sk_list_");
                out.push_str(&suffix);
                out.push_str("_new();\n");
                if let Expression::ListLiteral(items) = value.as_ref() {
                    for item in items {
                        out.push_str(&pad);
                        out.push_str("(void)sk_list_");
                        out.push_str(&suffix);
                        out.push_str("_push(&");
                        out.push_str(name);
                        out.push_str(", ");
                        out.push_str(&emit_expr(item, declared));
                        out.push_str(");\n");
                    }
                } else {
                    out.push_str(&pad);
                    out.push_str(name);
                    out.push_str(" = ");
                    out.push_str(&emit_expr(value, declared));
                    out.push_str(";\n");
                }
                declared.insert(name.clone(), dt.to_string());
                return;
            }
            out.push_str(&pad);
            if let Some(dt) = declared_type.as_deref()
                && let Expression::StructConstruction { fields } = value.as_ref()
            {
                out.push_str(&pad);
                out.push_str(&map_skadi_type_to_c(Some(dt)));
                out.push(' ');
                out.push_str(name);
                out.push_str(" = ");
                out.push_str(&emit_struct_literal(fields, Some(dt), declared));
                out.push_str(";\n");
                declared.insert(name.clone(), dt.to_string());
                return;
            }
            out.push_str(&map_skadi_type_to_c(declared_type.as_deref()));
            out.push(' ');
            out.push_str(name);
            out.push_str(" = ");
            out.push_str(&emit_expr(value, declared));
            out.push_str(";\n");
            declared.insert(name.clone(), declared_type.clone().unwrap_or_else(|| "Int".to_string()));
        }
        Statement::BlockStatement { statements, .. } | Statement::OnErrorBlock { statements, .. } => {
            let mut inner = declared.clone();
            for s in statements {
                emit_statement(s, out, indent, &mut inner, fn_ctx);
            }
        }
    }
}

fn map_skadi_type_to_c(skadi_type: Option<&str>) -> String {
    let normalized_owned = normalize_type_token(skadi_type.unwrap_or("Int"));
    let normalized = normalized_owned.as_str();
    match normalized {
        "i8" => "int8_t".to_string(),
        "i16" => "int16_t".to_string(),
        "i32" => "int32_t".to_string(),
        "Int" | "i64" => "int64_t".to_string(),
        "u8" => "uint8_t".to_string(),
        "u16" => "uint16_t".to_string(),
        "u32" => "uint32_t".to_string(),
        "u64" => "uint64_t".to_string(),
        "f32" => "float".to_string(),
        "Float" | "f64" => "double".to_string(),
        "bool" | "Bool" => "bool".to_string(),
        "char" | "Char" => "char".to_string(),
        "Text" | "Path" => "const char*".to_string(),
        other => other.to_string(),
    }
}

fn normalize_type_token(raw: &str) -> String {
    if let Some(elem) = raw.strip_suffix(" List") {
        let elem = elem.trim();
        let short = elem.rsplit('.').next().unwrap_or(elem);
        return format!("{} List", short);
    }
    raw.rsplit('.').next().unwrap_or(raw).to_string()
}

fn emit_struct_literal(
    fields: &std::collections::HashMap<String, Box<Expression>>,
    type_name: Option<&str>,
    declared: &HashMap<String, String>,
) -> String {
    let mut keys: Vec<&String> = fields.keys().collect();
    keys.sort();
    let mut body = String::new();
    let prefix = type_name
        .map(|t| format!("({})", normalize_type_token(t)))
        .unwrap_or_default();
    body.push_str(&prefix);
    body.push('{');
    for (i, k) in keys.iter().enumerate() {
        if i > 0 {
            body.push_str(", ");
        }
        body.push('.');
        body.push_str(k);
        body.push_str(" = ");
        if let Some(v) = fields.get(*k) {
            body.push_str(&emit_expr(v, declared));
        } else {
            body.push('0');
        }
    }
    body.push('}');
    body
}

fn is_text_expr(expr: &Expression, declared: &HashMap<String, String>) -> bool {
    match expr {
        Expression::LiteralString(_) => true,
        Expression::VariableReference(name) => declared
            .get(name)
            .map(|t| t.as_str() == "Text" || t.as_str() == "Path")
            .unwrap_or(false),
        Expression::Index { base, .. } => {
            if let Expression::VariableReference(name) = base.as_ref() {
                return declared
                    .get(name)
                    .and_then(|t| list_elem_from_decl(t))
                    .map(|elem| matches!(elem, "Text" | "Path"))
                    .unwrap_or(false);
            }
            false
        }
        Expression::MemberAccess { .. } => false,
        Expression::Call { name, .. } => matches!(
            name.as_str(),
            "input" | "read" | "slice" | "concat" | "fs.join"
        ),
        _ => false,
    }
}

fn emit_expr(expr: &Expression, declared: &HashMap<String, String>) -> String {
    match expr {
        Expression::LiteralInt(v) => v.to_string(),
        Expression::LiteralFloat(v) => v.to_string(),
        Expression::LiteralBool(v) => {
            if *v { "true".to_string() } else { "false".to_string() }
        }
        Expression::LiteralString(s) => s.clone(),
        Expression::VariableReference(name) => name.clone(),
        Expression::MemberAccess { base, field } => {
            if base == "my" {
                format!("my->{}", field)
            } else {
                format!("{}.{}", base, field)
            }
        }
        Expression::Index { base, index } => {
            let base_rendered = emit_expr(base, declared);
            let index_rendered = emit_expr(index, declared);
            if let Expression::VariableReference(name) = base.as_ref()
                && declared
                    .get(name)
                    .map(|t| t.as_str() == "Text")
                    .unwrap_or(false)
            {
                return format!("sk_text_char_at({}, {})", base_rendered, index_rendered);
            }
            if let Expression::VariableReference(name) = base.as_ref()
                && let Some(suffix) = declared
                    .get(name)
                    .and_then(|t| list_elem_from_decl(t))
                    .map(|elem| list_meta_dynamic(elem).1)
            {
                return format!("sk_list_{}_get(&{}, {})", suffix, base_rendered, index_rendered);
            }
            format!("{}.data[{}]", base_rendered, index_rendered)
        }
        Expression::Call { name, args } => {
            if let Some(builtin) = builtin_from_name(name) {
                match builtin {
                    Builtin::Len if args.len() == 1 => {
                        let arg_rendered = emit_expr(&args[0], declared);
                        if let Expression::VariableReference(var_name) = &args[0]
                            && declared
                                .get(var_name)
                                .map(|t| t.as_str() == "Text" || t.as_str() == "Path")
                                .unwrap_or(false)
                        {
                            return format!("((int64_t)strlen({}))", arg_rendered);
                        }
                        return format!("((int64_t){}.len)", arg_rendered);
                    }
                    Builtin::Contains if args.len() == 2 => {
                        let hay = emit_expr(&args[0], declared);
                        let needle = emit_expr(&args[1], declared);
                        return format!("(strstr({}, {}) != NULL)", hay, needle);
                    }
                    Builtin::Find if args.len() == 2 => {
                        let hay = emit_expr(&args[0], declared);
                        let needle = emit_expr(&args[1], declared);
                        return format!("sk_text_find({}, {})", hay, needle);
                    }
                    Builtin::Slice if args.len() == 3 => {
                        let text = emit_expr(&args[0], declared);
                        let start = emit_expr(&args[1], declared);
                        let end = emit_expr(&args[2], declared);
                        return format!("sk_text_slice({}, {}, {})", text, start, end);
                    }
                    Builtin::Concat if args.len() == 2 => {
                        let a = emit_expr(&args[0], declared);
                        let b = emit_expr(&args[1], declared);
                        return format!("sk_text_concat({}, {})", a, b);
                    }
                    Builtin::FsList if args.len() == 1 => {
                        let path = emit_expr(&args[0], declared);
                        return format!("sk_fs_list({})", path);
                    }
                    Builtin::FsIsDir if args.len() == 1 => {
                        let path = emit_expr(&args[0], declared);
                        return format!("sk_fs_is_dir({})", path);
                    }
                    Builtin::FsJoin if args.len() == 2 => {
                        let a = emit_expr(&args[0], declared);
                        let b = emit_expr(&args[1], declared);
                        return format!("sk_fs_join({}, {})", a, b);
                    }
                    Builtin::Args if args.is_empty() => {
                        return "sk_args(argc, argv)".to_string();
                    }
                    Builtin::Output if args.len() == 1 => {
                        let rendered = emit_expr(&args[0], declared);
                        if is_text_expr(&args[0], declared) {
                            return format!("sk_output_text({})", rendered);
                        }
                        return match args[0] {
                            Expression::LiteralFloat(_) => format!("sk_output_float({})", rendered),
                            Expression::LiteralBool(_) => format!("sk_output_bool({})", rendered),
                            Expression::LiteralString(_) => format!("sk_output_text({})", rendered),
                            Expression::LiteralInt(_) => format!("sk_output_int({})", rendered),
                            Expression::VariableReference(ref n) => {
                                match declared.get(n).map(String::as_str).unwrap_or("Int") {
                                    "Float" | "f32" | "f64" => format!("sk_output_float({})", rendered),
                                    "bool" | "Bool" => format!("sk_output_bool({})", rendered),
                                    "char" | "Char" => format!("sk_output_char({})", rendered),
                                    "Text" => format!("sk_output_text({})", rendered),
                                    _ => format!("sk_output_int({})", rendered),
                                }
                            }
                            _ => format!("sk_output_int({})", rendered),
                        };
                    }
                    Builtin::Input if args.len() == 1 => {
                        let prompt = emit_expr(&args[0], declared);
                        return format!("sk_input({})", prompt);
                    }
                    Builtin::Read if args.len() == 1 => {
                        let path = emit_expr(&args[0], declared);
                        return format!("sk_read_file({})", path);
                    }
                    Builtin::Write if args.len() == 2 => {
                        let path = emit_expr(&args[0], declared);
                        let data = emit_expr(&args[1], declared);
                        return format!("sk_write_file({}, {})", path, data);
                    }
                    _ => {}
                }
            }
            if let Some((obj, method)) = name.split_once(".")
                && let Some(obj_ty) = declared.get(obj)
                && let obj_ty_norm = normalize_type_token(obj_ty)
                && !matches!(
                    obj_ty_norm.as_str(),
                    "Int" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "Float"
                        | "f32" | "f64" | "bool" | "Bool" | "char" | "Char" | "Text" | "Path"
                )
                && !obj_ty_norm.ends_with(" List")
            {
                let mut rendered: Vec<String> = Vec::new();
                if obj == "my" {
                    rendered.push("my".to_string());
                } else {
                    rendered.push(format!("&{}", obj));
                }
                rendered.extend(args.iter().map(|a| emit_expr(a, declared)));
                return format!("{}_{}({})", obj_ty_norm, method, rendered.join(", "));
            }
            if let Some((base, method)) = name.split_once(".")
                && !declared.contains_key(base)
            {
                let rendered: Vec<String> = args.iter().map(|a| emit_expr(a, declared)).collect();
                return format!("{}({})", method, rendered.join(", "));
            }
            let rendered: Vec<String> = args.iter().map(|a| emit_expr(a, declared)).collect();
            format!("{}({})", name, rendered.join(", "))
        }
        Expression::BinaryOp { op, left, right } => {
            if op == "neg" {
                if let Some(r) = right {
                    return format!("(-{})", emit_expr(r, declared));
                }
                return format!("(-{})", emit_expr(left, declared));
            }
            if op == "not" {
                if let Some(r) = right {
                    return format!("(!{})", emit_expr(r, declared));
                }
                return format!("(!{})", emit_expr(left, declared));
            }
            let l = emit_expr(left, declared);
            if let Some(r) = right {
                let rr = emit_expr(r, declared);
                if (op == "==" || op == "!=") && is_text_expr(left, declared) && is_text_expr(r, declared) {
                    if op == "==" {
                        return format!("(strcmp({}, {}) == 0)", l, rr);
                    }
                    return format!("(strcmp({}, {}) != 0)", l, rr);
                }
                let c_op = match op.as_str() {
                    "and" => "&&",
                    "or" => "||",
                    "xor" => "^",
                    "div" => "/",
                    "mod" => "%",
                    other => other,
                };
                format!("({} {} {})", l, c_op, rr)
            } else {
                format!("({})", l)
            }
        }
        Expression::StructConstruction { fields } => emit_struct_literal(fields, None, declared),
        Expression::ListLiteral(_) => "0 /* TODO(v1): list literal */".to_string(),
    }
}
