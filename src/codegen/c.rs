use std::collections::{HashMap, HashSet};

use crate::ast_nodes::{BlockStatement, Expression, Program, Statement};
use crate::builtins::{Builtin, builtin_from_name};

struct FunctionContext {
    is_danger: bool,
    return_type: Option<String>,
}

#[derive(Clone, Debug)]
struct PlaceContext {
    memory_name: String,
    restore_region_var: String,
    fail_label: String,
}

#[derive(Default)]
struct CodegenState {
    next_label_id: usize,
}

impl CodegenState {
    fn next_id(&mut self) -> usize {
        let id = self.next_label_id;
        self.next_label_id += 1;
        id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExprKind {
    Int,
    Float,
    Bool,
    Char,
    Text,
    Unknown,
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
    ("Text", "char*", "text"),
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

fn channel_elem_from_decl(declared_type: &str) -> Option<&str> {
    let base_type = declared_type
        .strip_suffix("@owned")
        .unwrap_or(declared_type);
    base_type
        .strip_prefix("Channel(")
        .and_then(|inner| inner.strip_suffix(')'))
        .map(str::trim)
}

fn channel_type_suffix(skadi_type: &str) -> String {
    skadi_type
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
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

fn emit_thread_local_support(out: &mut String) {
    out.push_str("#if defined(_MSC_VER)\n");
    out.push_str("#define SK_THREAD_LOCAL __declspec(thread)\n");
    out.push_str("#else\n");
    out.push_str("#define SK_THREAD_LOCAL _Thread_local\n");
    out.push_str("#endif\n\n");
}

fn emit_memory_runtime(out: &mut String) {
    out.push_str("typedef struct SkMemoryRegion {\n");
    out.push_str("    unsigned char *buffer;\n");
    out.push_str("    size_t capacity;\n");
    out.push_str("    size_t offset;\n");
    out.push_str("    bool failed;\n");
    out.push_str("} SkMemoryRegion;\n\n");
    out.push_str("typedef union {\n");
    out.push_str("    long double long_double_value;\n");
    out.push_str("    void *pointer_value;\n");
    out.push_str("    int64_t integer_value;\n");
    out.push_str("} SkMemoryAlignment;\n\n");
    out.push_str("typedef struct SkAllocHeader {\n");
    out.push_str("    uint32_t magic;\n");
    out.push_str("    SkMemoryRegion *owner_region;\n");
    out.push_str("    size_t size;\n");
    out.push_str("    SkMemoryAlignment alignment;\n");
    out.push_str("} SkAllocHeader;\n\n");
    out.push_str("#define SK_ALLOC_MAGIC 0x534B4144u\n\n");
    out.push_str("static SK_THREAD_LOCAL SkMemoryRegion *sk_active_region = NULL;\n\n");
    out.push_str("static size_t sk_mem_align_up(size_t value, size_t alignment) {\n");
    out.push_str("    size_t rem = value % alignment;\n");
    out.push_str("    return rem == 0 ? value : (value + (alignment - rem));\n");
    out.push_str("}\n\n");
    out.push_str("static SkAllocHeader* sk_header_from_ptr(const void *ptr) {\n");
    out.push_str("    if (!ptr) return NULL;\n");
    out.push_str("    SkAllocHeader *header = ((SkAllocHeader*)ptr) - 1;\n");
    out.push_str("    if (header->magic != SK_ALLOC_MAGIC) return NULL;\n");
    out.push_str("    return header;\n");
    out.push_str("}\n\n");
    out.push_str("static SkMemoryRegion* sk_mem_set_active(SkMemoryRegion *region) {\n");
    out.push_str("    SkMemoryRegion *previous = sk_active_region;\n");
    out.push_str("    sk_active_region = region;\n");
    out.push_str("    return previous;\n");
    out.push_str("}\n\n");
    out.push_str("static SkMemoryRegion* sk_mem_current(void) {\n");
    out.push_str("    return sk_active_region;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_mem_clear_failure(SkMemoryRegion *region) {\n");
    out.push_str("    if (region) region->failed = false;\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_mem_failed(SkMemoryRegion *region) {\n");
    out.push_str("    return region && region->failed;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_mem_panic(const char *message) {\n");
    out.push_str("    fprintf(stderr, \"Skadi memory runtime error: %s\\n\", message ? message : \"unknown\");\n");
    out.push_str("    exit(1);\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_mem_region_init(SkMemoryRegion *region, size_t capacity) {\n");
    out.push_str("    if (!region) return false;\n");
    out.push_str("    region->buffer = NULL;\n");
    out.push_str("    region->capacity = capacity;\n");
    out.push_str("    region->offset = 0;\n");
    out.push_str("    region->failed = false;\n");
    out.push_str("    if (capacity == 0) return true;\n");
    out.push_str("    region->buffer = (unsigned char*)malloc(capacity);\n");
    out.push_str("    return region->buffer != NULL;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_mem_region_clear(SkMemoryRegion *region) {\n");
    out.push_str("    if (!region) return;\n");
    out.push_str("    region->offset = 0;\n");
    out.push_str("    region->failed = false;\n");
    out.push_str("}\n\n");
    out.push_str("static void* sk_alloc_bytes_in(SkMemoryRegion *region, size_t size) {\n");
    out.push_str("    size_t total = sizeof(SkAllocHeader) + size;\n");
    out.push_str("    if (region) {\n");
    out.push_str(
        "        size_t start = sk_mem_align_up(region->offset, sizeof(SkMemoryAlignment));\n",
    );
    out.push_str("        if (!region->buffer || start + total > region->capacity) {\n");
    out.push_str("            region->failed = true;\n");
    out.push_str("            return NULL;\n");
    out.push_str("        }\n");
    out.push_str("        SkAllocHeader *header = (SkAllocHeader*)(region->buffer + start);\n");
    out.push_str("        header->magic = SK_ALLOC_MAGIC;\n");
    out.push_str("        header->owner_region = region;\n");
    out.push_str("        header->size = size;\n");
    out.push_str("        region->offset = start + total;\n");
    out.push_str("        return (void*)(header + 1);\n");
    out.push_str("    }\n");
    out.push_str("    SkAllocHeader *header = (SkAllocHeader*)malloc(total);\n");
    out.push_str("    if (!header) return NULL;\n");
    out.push_str("    header->magic = SK_ALLOC_MAGIC;\n");
    out.push_str("    header->owner_region = NULL;\n");
    out.push_str("    header->size = size;\n");
    out.push_str("    return (void*)(header + 1);\n");
    out.push_str("}\n\n");
    out.push_str("static void* sk_alloc_bytes(size_t size) {\n");
    out.push_str("    return sk_alloc_bytes_in(sk_mem_current(), size);\n");
    out.push_str("}\n\n");
    out.push_str("static char* sk_text_alloc(size_t size) {\n");
    out.push_str("    return (char*)sk_alloc_bytes(size + 1);\n");
    out.push_str("}\n\n");
    out.push_str("static char* sk_text_dup(const char *s) {\n");
    out.push_str("    const char *src = s ? s : \"\";\n");
    out.push_str("    size_t n = strlen(src);\n");
    out.push_str("    char *out = sk_text_alloc(n);\n");
    out.push_str("    if (!out) return NULL;\n");
    out.push_str("    memcpy(out, src, n);\n");
    out.push_str("    out[n] = '\\0';\n");
    out.push_str("    return out;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_free_text(void *ptr) {\n");
    out.push_str("    SkAllocHeader *header = sk_header_from_ptr(ptr);\n");
    out.push_str("    if (!header) return;\n");
    out.push_str("    if (header->owner_region) return;\n");
    out.push_str("    free(header);\n");
    out.push_str("}\n\n");
}

fn emit_list_helpers_for(out: &mut String, c_ty: &str, suffix: &str) {
    out.push_str(&format!(
        "typedef struct {{\n    {} *data;\n    size_t len;\n    size_t cap;\n    SkMemoryRegion *owner_region;\n}} SkadiList_{};\n\n",
        c_ty, suffix
    ));
    out.push_str(&format!(
        "static SkadiList_{} sk_list_{}_new(void) {{\n",
        suffix, suffix
    ));
    out.push_str(&format!("    SkadiList_{} xs;\n", suffix));
    out.push_str("    xs.data = NULL;\n");
    out.push_str("    xs.len = 0;\n");
    out.push_str("    xs.cap = 0;\n");
    out.push_str("    xs.owner_region = sk_mem_current();\n");
    out.push_str("    return xs;\n");
    out.push_str("}\n\n");
    out.push_str(&format!(
        "static int sk_list_{}_push(SkadiList_{} *xs, {} v) {{\n",
        suffix, suffix, c_ty
    ));
    out.push_str("    if (xs->len == xs->cap) {\n");
    out.push_str("        size_t next = xs->cap == 0 ? 4 : xs->cap * 2;\n");
    out.push_str("        size_t bytes = next * sizeof(*xs->data);\n");
    out.push_str("        if (xs->owner_region) {\n");
    out.push_str("            void *raw = sk_alloc_bytes_in(xs->owner_region, bytes);\n");
    out.push_str("            if (!raw) return 1;\n");
    out.push_str("            if (xs->data && xs->len > 0) memcpy(raw, xs->data, xs->len * sizeof(*xs->data));\n");
    out.push_str(&format!("            xs->data = ({c_ty}*)raw;\n"));
    out.push_str("        } else {\n");
    out.push_str("            SkAllocHeader *header = sk_header_from_ptr(xs->data);\n");
    out.push_str("            size_t total = sizeof(SkAllocHeader) + bytes;\n");
    out.push_str("            if (header) {\n");
    out.push_str("                header = (SkAllocHeader*)realloc(header, total);\n");
    out.push_str("                if (!header) return 1;\n");
    out.push_str("                header->magic = SK_ALLOC_MAGIC;\n");
    out.push_str("                header->owner_region = NULL;\n");
    out.push_str("                header->size = bytes;\n");
    out.push_str("                xs->data = ");
    out.push_str(&format!("({c_ty}*)(header + 1);\n"));
    out.push_str("            } else {\n");
    out.push_str("                void *raw = sk_alloc_bytes_in(NULL, bytes);\n");
    out.push_str("                if (!raw) return 1;\n");
    out.push_str(&format!("                xs->data = ({c_ty}*)raw;\n"));
    out.push_str("            }\n");
    out.push_str("        }\n");
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
        "static void sk_list_{}_free(SkadiList_{} *xs) {{\n",
        suffix, suffix
    ));
    out.push_str("    if (!xs) return;\n");
    out.push_str("    if (!xs->owner_region) {\n");
    out.push_str("        SkAllocHeader *header = sk_header_from_ptr(xs->data);\n");
    out.push_str("        if (header) free(header);\n");
    out.push_str("    }\n");
    out.push_str("    xs->data = NULL;\n");
    out.push_str("    xs->len = 0;\n");
    out.push_str("    xs->cap = 0;\n");
    out.push_str("    xs->owner_region = NULL;\n");
    out.push_str("}\n\n");
    if suffix == "text" {
        out.push_str("static void sk_list_text_free_owned(SkadiList_text *xs) {\n");
        out.push_str("    if (!xs) return;\n");
        out.push_str(
            "    for (size_t i = 0; i < xs->len; ++i) sk_free_text((void*)xs->data[i]);\n",
        );
        out.push_str("    sk_list_text_free(xs);\n");
        out.push_str("}\n\n");
    }
    out.push_str(&format!(
        "static {} sk_list_{}_get(const SkadiList_{} *xs, int64_t idx) {{\n",
        c_ty, suffix, suffix
    ));
    let fallback = if LIST_TYPE_MAP
        .iter()
        .any(|(_, mapped_ty, _)| *mapped_ty == c_ty)
    {
        "0".to_string()
    } else {
        format!("({}){{0}}", c_ty)
    };
    out.push_str(&format!(
        "    if (!xs || idx < 0 || (size_t)idx >= xs->len) return {};\n",
        fallback
    ));
    out.push_str("    return xs->data[(size_t)idx];\n");
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
    out.push_str("    char *out = sk_text_alloc(len);\n");
    out.push_str("    if (!out) return NULL;\n");
    out.push_str("    if (len > 0) {\n");
    out.push_str("        memcpy(out, s + start, len);\n");
    out.push_str("    }\n");
    out.push_str("    out[len] = '\\0';\n");
    out.push_str("    return out;\n");
    out.push_str("}\n\n");
    out.push_str("static char* sk_text_concat(const char *a, const char *b) {\n");
    out.push_str("    const char *left = a ? a : \"\";\n");
    out.push_str("    const char *right = b ? b : \"\";\n");
    out.push_str("    size_t alen = strlen(left);\n");
    out.push_str("    size_t blen = strlen(right);\n");
    out.push_str("    char *out = sk_text_alloc(alen + blen);\n");
    out.push_str("    if (!out) return NULL;\n");
    out.push_str("    memcpy(out, left, alen);\n");
    out.push_str("    memcpy(out + alen, right, blen);\n");
    out.push_str("    out[alen + blen] = '\\0';\n");
    out.push_str("    return out;\n");
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
        out.push_str("        char *name = sk_text_dup(ent->d_name);\n");
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
        out.push_str(
            "    bool need_sep = alen > 0 && left[alen - 1] != '/' && left[alen - 1] != '\\\\';\n",
        );
        out.push_str("    size_t n = alen + (need_sep ? 1 : 0) + blen;\n");
        out.push_str("    char *outp = sk_text_alloc(n);\n");
        out.push_str("    if (!outp) return NULL;\n");
        out.push_str("    memcpy(outp, left, alen);\n");
        out.push_str("    size_t p = alen;\n");
        out.push_str("    if (need_sep) outp[p++] = '/';\n");
        out.push_str("    memcpy(outp + p, right, blen);\n");
        out.push_str("    outp[n] = '\\0';\n");
        out.push_str("    return outp;\n");
        out.push_str("}\n\n");
    }
}

fn emit_io_runtime(out: &mut String, needs_args_runtime: bool) {
    out.push_str(
        "static int sk_output_text(const char *s) { printf(\"%s\\n\", s ? s : \"\"); return 0; }\n",
    );
    out.push_str(
        "static int sk_output_int(int64_t v) { printf(\"%lld\\n\", (long long)v); return 0; }\n",
    );
    out.push_str("static int sk_output_float(double v) { printf(\"%f\\n\", v); return 0; }\n");
    out.push_str("static int sk_output_bool(bool v) { printf(\"%s\\n\", v ? \"true\" : \"false\"); return 0; }\n");
    out.push_str("static int sk_output_char(char v) { printf(\"%c\\n\", v); return 0; }\n\n");
    out.push_str("static char* sk_input(const char *prompt) {\n");
    out.push_str("    if (prompt) printf(\"%s\", prompt);\n");
    out.push_str("    char buf[4096];\n");
    out.push_str("    if (!fgets(buf, sizeof(buf), stdin)) return sk_text_dup(\"\");\n");
    out.push_str("    size_t n = strlen(buf);\n");
    out.push_str("    if (n > 0 && buf[n - 1] == '\\n') buf[n - 1] = '\\0';\n");
    out.push_str("    return sk_text_dup(buf);\n");
    out.push_str("}\n\n");
    out.push_str("static char* sk_read_file(const char *path) {\n");
    out.push_str("    FILE *f = fopen(path, \"rb\");\n");
    out.push_str("    if (!f) return sk_text_dup(\"\");\n");
    out.push_str("    fseek(f, 0, SEEK_END);\n");
    out.push_str("    long n = ftell(f);\n");
    out.push_str("    fseek(f, 0, SEEK_SET);\n");
    out.push_str("    if (n < 0) { fclose(f); return sk_text_dup(\"\"); }\n");
    out.push_str("    char *buf = sk_text_alloc((size_t)n);\n");
    out.push_str("    if (!buf) { fclose(f); return NULL; }\n");
    out.push_str("    size_t r = fread(buf, 1, (size_t)n, f);\n");
    out.push_str("    buf[r] = '\\0';\n");
    out.push_str("    fclose(f);\n");
    out.push_str("    return buf;\n");
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
        out.push_str("        char *v = sk_text_dup(argv[i] ? argv[i] : \"\");\n");
        out.push_str("        if (!v) continue;\n");
        out.push_str("        (void)sk_list_text_push(&out, v);\n");
        out.push_str("    }\n");
        out.push_str("    return out;\n");
        out.push_str("}\n\n");
    }
}

fn emit_math_runtime(out: &mut String) {
    out.push_str("#ifndef M_PI\n#define M_PI 3.14159265358979323846\n#endif\n");
    out.push_str("#ifndef M_E\n#define M_E 2.71828182845904523536\n#endif\n\n");
}

fn map_function_name(name: &str) -> &str {
    if name == "main" {
        "skadi_user_main"
    } else {
        name
    }
}

fn has_user_main(program: &Program) -> bool {
    program
        .statements
        .iter()
        .any(|stmt| matches!(stmt, Statement::FunctionDef { name, .. } if name == "main"))
}

fn function_uses_memory_surface(stmt: &Statement) -> bool {
    match stmt {
        Statement::FunctionDef { params, body, .. } => {
            params
                .iter()
                .any(|param| param.param_type.as_deref() == Some("Memory"))
                || statement_list_uses_memory_surface(&body.statements)
        }
        Statement::StructDecl { methods, .. } => methods.iter().any(|method| {
            method
                .params
                .iter()
                .any(|param| param.param_type.as_deref() == Some("Memory"))
                || statement_list_uses_memory_surface(&method.body.statements)
        }),
        _ => false,
    }
}

fn statement_list_uses_memory_surface(statements: &[Statement]) -> bool {
    statements.iter().any(statement_uses_memory_surface)
}

fn statement_uses_memory_surface(stmt: &Statement) -> bool {
    match stmt {
        Statement::MemoryDecl { .. }
        | Statement::PlaceIn { .. }
        | Statement::MemoryClear { .. } => true,
        Statement::FunctionDef { .. } | Statement::StructDecl { .. } => {
            function_uses_memory_surface(stmt)
        }
        Statement::IfStatement {
            then_block,
            else_block,
            ..
        } => {
            statement_list_uses_memory_surface(&then_block.statements)
                || else_block
                    .as_ref()
                    .map(|block| statement_list_uses_memory_surface(&block.statements))
                    .unwrap_or(false)
        }
        Statement::ForLoop { body, .. }
        | Statement::WhileLoop { body, .. }
        | Statement::LoopStatement { body, .. } => {
            statement_list_uses_memory_surface(&body.statements)
        }
        Statement::WhenBlock {
            cases, else_block, ..
        } => {
            cases
                .iter()
                .any(|(_, block)| statement_list_uses_memory_surface(&block.statements))
                || else_block
                    .as_ref()
                    .map(|block| statement_list_uses_memory_surface(&block.statements))
                    .unwrap_or(false)
        }
        Statement::OnBlock { body, .. } => statement_list_uses_memory_surface(&body.statements),
        Statement::DangerAssignOnError { on_error, .. }
        | Statement::DangerCallOnError { on_error, .. }
        | Statement::ListPopOnError { on_error, .. } => {
            statement_list_uses_memory_surface(&on_error.statements)
        }
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => {
            statement_list_uses_memory_surface(statements)
        }
        _ => false,
    }
}

fn expression_uses_task_surface(expr: &Expression) -> bool {
    match expr {
        Expression::RunTask { .. } | Expression::WaitTask { .. } | Expression::Stopping => true,
        Expression::Call { args, .. } => args.iter().any(expression_uses_task_surface),
        Expression::BinaryOp { left, right, .. } => {
            expression_uses_task_surface(left)
                || right
                    .as_deref()
                    .map(expression_uses_task_surface)
                    .unwrap_or(false)
        }
        Expression::Index { base, index } => {
            expression_uses_task_surface(base) || expression_uses_task_surface(index)
        }
        Expression::ListLiteral(items) => items.iter().any(expression_uses_task_surface),
        Expression::StructConstruction { fields } => fields
            .values()
            .any(|value| expression_uses_task_surface(value)),
        Expression::VariableReference(_)
        | Expression::MemberAccess { .. }
        | Expression::LiteralInt(_)
        | Expression::LiteralFloat(_)
        | Expression::LiteralBool(_)
        | Expression::LiteralString(_) => false,
    }
}

fn statement_uses_task_surface(stmt: &Statement) -> bool {
    match stmt {
        Statement::VarDecl {
            declared_type,
            value,
            ..
        } => {
            declared_type
                .as_deref()
                .map(|ty| ty == "Task" || ty.starts_with("Task("))
                .unwrap_or(false)
                || expression_uses_task_surface(value)
        }
        Statement::StopTask { .. } => true,
        Statement::Assignment { value, .. }
        | Statement::FieldAssignment { value, .. }
        | Statement::ListPush { value, .. } => expression_uses_task_surface(value),
        Statement::ReturnStatement { value, .. } => value
            .as_deref()
            .map(expression_uses_task_surface)
            .unwrap_or(false),
        Statement::ExpressionStatement { expr, .. } => expression_uses_task_surface(expr),
        Statement::FunctionDef { params, body, .. } => {
            params.iter().any(|param| {
                param
                    .param_type
                    .as_deref()
                    .map(|ty| ty == "Task" || ty.starts_with("Task("))
                    .unwrap_or(false)
            }) || statement_list_uses_task_surface(&body.statements)
        }
        Statement::StructDecl {
            fields, methods, ..
        } => {
            fields
                .iter()
                .any(|field| field.field_type == "Task" || field.field_type.starts_with("Task("))
                || methods
                    .iter()
                    .any(|method| statement_list_uses_task_surface(&method.body.statements))
        }
        Statement::IfStatement {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_uses_task_surface(condition)
                || statement_list_uses_task_surface(&then_block.statements)
                || else_block
                    .as_ref()
                    .map(|block| statement_list_uses_task_surface(&block.statements))
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
                .as_deref()
                .map(expression_uses_task_surface)
                .unwrap_or(false)
                || condition
                    .as_deref()
                    .map(expression_uses_task_surface)
                    .unwrap_or(false)
                || update
                    .as_deref()
                    .map(expression_uses_task_surface)
                    .unwrap_or(false)
                || statement_list_uses_task_surface(&body.statements)
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            expression_uses_task_surface(when_expression)
                || cases.iter().any(|(exprs, block)| {
                    exprs.iter().any(expression_uses_task_surface)
                        || statement_list_uses_task_surface(&block.statements)
                })
                || else_block
                    .as_ref()
                    .map(|block| statement_list_uses_task_surface(&block.statements))
                    .unwrap_or(false)
        }
        Statement::WhileLoop {
            condition, body, ..
        } => {
            expression_uses_task_surface(condition)
                || statement_list_uses_task_surface(&body.statements)
        }
        Statement::LoopStatement { body, .. } | Statement::OnBlock { body, .. } => {
            statement_list_uses_task_surface(&body.statements)
        }
        Statement::PlaceIn { body, on_error, .. } => {
            statement_list_uses_task_surface(&body.statements)
                || on_error
                    .as_ref()
                    .map(|block| statement_list_uses_task_surface(&block.statements))
                    .unwrap_or(false)
        }
        Statement::MemoryDecl { on_error, .. } => on_error
            .as_ref()
            .map(|block| statement_list_uses_task_surface(&block.statements))
            .unwrap_or(false),
        Statement::DangerAssignOnError { on_error, .. }
        | Statement::DangerCallOnError { on_error, .. }
        | Statement::ListPopOnError { on_error, .. } => {
            statement_list_uses_task_surface(&on_error.statements)
        }
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => {
            statement_list_uses_task_surface(statements)
        }
        Statement::MemoryClear { .. }
        | Statement::ReturnError { .. }
        | Statement::IncDec { .. }
        | Statement::BreakStatement { .. }
        | Statement::ContinueStatement { .. }
        | Statement::PassStatement { .. }
        | Statement::LabelDecl { .. } => false,
    }
}

fn statement_list_uses_task_surface(statements: &[Statement]) -> bool {
    statements.iter().any(statement_uses_task_surface)
}

fn expression_uses_deferred_task_surface(expr: &Expression) -> bool {
    match expr {
        Expression::Call { name, args } => {
            name == "channel"
                || name.ends_with(".send")
                || name.ends_with(".receive")
                || args.iter().any(expression_uses_deferred_task_surface)
        }
        Expression::RunTask { args, .. } => args.iter().any(expression_uses_deferred_task_surface),
        Expression::WaitTask { .. } | Expression::Stopping => false,
        Expression::BinaryOp { left, right, .. } => {
            expression_uses_deferred_task_surface(left)
                || right
                    .as_deref()
                    .map(expression_uses_deferred_task_surface)
                    .unwrap_or(false)
        }
        Expression::Index { base, index } => {
            expression_uses_deferred_task_surface(base)
                || expression_uses_deferred_task_surface(index)
        }
        Expression::ListLiteral(items) => items.iter().any(expression_uses_deferred_task_surface),
        Expression::StructConstruction { fields } => fields
            .values()
            .any(|value| expression_uses_deferred_task_surface(value)),
        _ => false,
    }
}

fn statement_uses_deferred_task_surface(stmt: &Statement) -> bool {
    match stmt {
        Statement::VarDecl {
            declared_type,
            value,
            ..
        } => {
            declared_type
                .as_deref()
                .map(|ty| ty.starts_with("Channel("))
                .unwrap_or(false)
                || expression_uses_deferred_task_surface(value)
        }
        Statement::StopTask { .. } => false,
        Statement::Assignment { value, .. }
        | Statement::FieldAssignment { value, .. }
        | Statement::ListPush { value, .. } => expression_uses_deferred_task_surface(value),
        Statement::ReturnStatement { value, .. } => value
            .as_deref()
            .map(expression_uses_deferred_task_surface)
            .unwrap_or(false),
        Statement::ExpressionStatement { expr, .. } => expression_uses_deferred_task_surface(expr),
        Statement::FunctionDef { params, body, .. } => {
            params.iter().any(|param| {
                param
                    .param_type
                    .as_deref()
                    .map(|ty| ty.starts_with("Channel("))
                    .unwrap_or(false)
            }) || body
                .statements
                .iter()
                .any(statement_uses_deferred_task_surface)
        }
        Statement::StructDecl {
            fields, methods, ..
        } => {
            fields
                .iter()
                .any(|field| field.field_type.starts_with("Channel("))
                || methods.iter().any(|method| {
                    method
                        .body
                        .statements
                        .iter()
                        .any(statement_uses_deferred_task_surface)
                })
        }
        Statement::IfStatement {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_uses_deferred_task_surface(condition)
                || then_block
                    .statements
                    .iter()
                    .any(statement_uses_deferred_task_surface)
                || else_block
                    .as_ref()
                    .map(|block| {
                        block
                            .statements
                            .iter()
                            .any(statement_uses_deferred_task_surface)
                    })
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
                .as_deref()
                .map(expression_uses_deferred_task_surface)
                .unwrap_or(false)
                || condition
                    .as_deref()
                    .map(expression_uses_deferred_task_surface)
                    .unwrap_or(false)
                || update
                    .as_deref()
                    .map(expression_uses_deferred_task_surface)
                    .unwrap_or(false)
                || body
                    .statements
                    .iter()
                    .any(statement_uses_deferred_task_surface)
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            expression_uses_deferred_task_surface(when_expression)
                || cases.iter().any(|(expressions, block)| {
                    expressions
                        .iter()
                        .any(expression_uses_deferred_task_surface)
                        || block
                            .statements
                            .iter()
                            .any(statement_uses_deferred_task_surface)
                })
                || else_block
                    .as_ref()
                    .map(|block| {
                        block
                            .statements
                            .iter()
                            .any(statement_uses_deferred_task_surface)
                    })
                    .unwrap_or(false)
        }
        Statement::WhileLoop {
            condition, body, ..
        } => {
            expression_uses_deferred_task_surface(condition)
                || body
                    .statements
                    .iter()
                    .any(statement_uses_deferred_task_surface)
        }
        Statement::LoopStatement { body, .. } | Statement::OnBlock { body, .. } => body
            .statements
            .iter()
            .any(statement_uses_deferred_task_surface),
        Statement::PlaceIn { body, on_error, .. } => {
            body.statements
                .iter()
                .any(statement_uses_deferred_task_surface)
                || on_error
                    .as_ref()
                    .map(|block| {
                        block
                            .statements
                            .iter()
                            .any(statement_uses_deferred_task_surface)
                    })
                    .unwrap_or(false)
        }
        Statement::MemoryDecl { on_error, .. } => on_error
            .as_ref()
            .map(|block| {
                block
                    .statements
                    .iter()
                    .any(statement_uses_deferred_task_surface)
            })
            .unwrap_or(false),
        Statement::DangerAssignOnError { on_error, .. }
        | Statement::DangerCallOnError { on_error, .. }
        | Statement::ListPopOnError { on_error, .. } => on_error
            .statements
            .iter()
            .any(statement_uses_deferred_task_surface),
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => {
            statements.iter().any(statement_uses_deferred_task_surface)
        }
        _ => false,
    }
}

fn statement_list_uses_deferred_task_surface(statements: &[Statement]) -> bool {
    statements.iter().any(statement_uses_deferred_task_surface)
}

pub fn ensure_codegen_supported(program: &Program) -> Result<(), String> {
    let _ = program;
    Ok(())
}

fn collect_task_entries_from_expression(expr: &Expression, entries: &mut HashSet<String>) {
    match expr {
        Expression::RunTask { call_name, args } => {
            entries.insert(call_name.clone());
            for arg in args {
                collect_task_entries_from_expression(arg, entries);
            }
        }
        Expression::Call { args, .. } | Expression::ListLiteral(args) => {
            for arg in args {
                collect_task_entries_from_expression(arg, entries);
            }
        }
        Expression::Index { base, index } => {
            collect_task_entries_from_expression(base, entries);
            collect_task_entries_from_expression(index, entries);
        }
        Expression::BinaryOp { left, right, .. } => {
            collect_task_entries_from_expression(left, entries);
            if let Some(right) = right {
                collect_task_entries_from_expression(right, entries);
            }
        }
        Expression::StructConstruction { fields } => {
            for value in fields.values() {
                collect_task_entries_from_expression(value, entries);
            }
        }
        _ => {}
    }
}

fn collect_task_entries_from_statements(statements: &[Statement], entries: &mut HashSet<String>) {
    for stmt in statements {
        match stmt {
            Statement::VarDecl { value, .. }
            | Statement::Assignment { value, .. }
            | Statement::FieldAssignment { value, .. }
            | Statement::ListPush { value, .. }
            | Statement::ExpressionStatement { expr: value, .. } => {
                collect_task_entries_from_expression(value, entries);
            }
            Statement::ReturnStatement {
                value: Some(value), ..
            } => collect_task_entries_from_expression(value, entries),
            Statement::FunctionDef { body, .. } => {
                collect_task_entries_from_statements(&body.statements, entries)
            }
            Statement::StructDecl { methods, .. } => {
                for method in methods {
                    collect_task_entries_from_statements(&method.body.statements, entries);
                }
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                collect_task_entries_from_expression(condition, entries);
                collect_task_entries_from_statements(&then_block.statements, entries);
                if let Some(else_block) = else_block {
                    collect_task_entries_from_statements(&else_block.statements, entries);
                }
            }
            Statement::ForLoop {
                initialization,
                condition,
                update,
                body,
                ..
            } => {
                if let Some(expression) = initialization {
                    collect_task_entries_from_expression(expression, entries);
                }
                if let Some(expression) = condition {
                    collect_task_entries_from_expression(expression, entries);
                }
                if let Some(expression) = update {
                    collect_task_entries_from_expression(expression, entries);
                }
                collect_task_entries_from_statements(&body.statements, entries);
            }
            Statement::WhileLoop {
                condition, body, ..
            } => {
                collect_task_entries_from_expression(condition, entries);
                collect_task_entries_from_statements(&body.statements, entries);
            }
            Statement::LoopStatement { body, .. } | Statement::OnBlock { body, .. } => {
                collect_task_entries_from_statements(&body.statements, entries)
            }
            Statement::WhenBlock {
                when_expression,
                cases,
                else_block,
                ..
            } => {
                collect_task_entries_from_expression(when_expression, entries);
                for (expressions, block) in cases {
                    for expression in expressions {
                        collect_task_entries_from_expression(expression, entries);
                    }
                    collect_task_entries_from_statements(&block.statements, entries);
                }
                if let Some(else_block) = else_block {
                    collect_task_entries_from_statements(&else_block.statements, entries);
                }
            }
            Statement::PlaceIn { body, on_error, .. } => {
                collect_task_entries_from_statements(&body.statements, entries);
                if let Some(on_error) = on_error {
                    collect_task_entries_from_statements(&on_error.statements, entries);
                }
            }
            Statement::MemoryDecl {
                on_error: Some(on_error),
                ..
            } => collect_task_entries_from_statements(&on_error.statements, entries),
            Statement::DangerAssignOnError { args, on_error, .. }
            | Statement::DangerCallOnError { args, on_error, .. } => {
                for arg in args {
                    collect_task_entries_from_expression(arg, entries);
                }
                collect_task_entries_from_statements(&on_error.statements, entries);
            }
            Statement::ListPopOnError { on_error, .. } => {
                collect_task_entries_from_statements(&on_error.statements, entries)
            }
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => {
                collect_task_entries_from_statements(statements, entries)
            }
            _ => {}
        }
    }
}

fn collect_task_entries(program: &Program) -> HashSet<String> {
    let mut entries = HashSet::new();
    collect_task_entries_from_statements(&program.statements, &mut entries);
    entries
}

fn emit_task_runtime(out: &mut String) {
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("typedef HANDLE SkPlatformThread;\n");
    out.push_str("#else\n");
    out.push_str("typedef pthread_t SkPlatformThread;\n");
    out.push_str("#endif\n\n");
    out.push_str("typedef struct SkTask SkTask;\n");
    out.push_str("typedef void (*SkTaskEntry)(SkTask *task, void *context);\n\n");
    out.push_str("struct SkTask {\n");
    out.push_str("    SkPlatformThread thread;\n");
    out.push_str("    void *context;\n");
    out.push_str("    bool started;\n");
    out.push_str("    bool joined;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    volatile LONG stop_requested;\n");
    out.push_str("#else\n");
    out.push_str("    pthread_mutex_t stop_mutex;\n");
    out.push_str("    bool stop_requested;\n");
    out.push_str("#endif\n");
    out.push_str("};\n\n");
    out.push_str("static SK_THREAD_LOCAL SkTask *sk_current_task = NULL;\n\n");
    out.push_str("typedef struct {\n");
    out.push_str("    SkTask *task;\n");
    out.push_str("    SkTaskEntry entry;\n");
    out.push_str("} SkTaskLaunch;\n\n");
    out.push_str("static void sk_task_panic(const char *code, const char *message) {\n");
    out.push_str("    fprintf(stderr, \"Runtime error: [%s] %s\\n\", code, message);\n");
    out.push_str("    exit(1);\n");
    out.push_str("}\n\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("static DWORD WINAPI sk_task_platform_entry(LPVOID raw) {\n");
    out.push_str("    SkTaskLaunch *launch = (SkTaskLaunch*)raw;\n");
    out.push_str("    SkTask *task = launch->task;\n");
    out.push_str("    SkTaskEntry entry = launch->entry;\n");
    out.push_str("    free(launch);\n");
    out.push_str("    sk_current_task = task;\n");
    out.push_str("    entry(task, task->context);\n");
    out.push_str("    sk_current_task = NULL;\n");
    out.push_str("    return 0;\n");
    out.push_str("}\n");
    out.push_str("#else\n");
    out.push_str("static void* sk_task_platform_entry(void *raw) {\n");
    out.push_str("    SkTaskLaunch *launch = (SkTaskLaunch*)raw;\n");
    out.push_str("    SkTask *task = launch->task;\n");
    out.push_str("    SkTaskEntry entry = launch->entry;\n");
    out.push_str("    free(launch);\n");
    out.push_str("    sk_current_task = task;\n");
    out.push_str("    entry(task, task->context);\n");
    out.push_str("    sk_current_task = NULL;\n");
    out.push_str("    return NULL;\n");
    out.push_str("}\n");
    out.push_str("#endif\n\n");
    out.push_str("static bool sk_task_start(SkTask *task, SkTaskEntry entry, void *context) {\n");
    out.push_str("    if (!task || !entry || !context) return false;\n");
    out.push_str("    task->context = context;\n");
    out.push_str("    task->started = false;\n");
    out.push_str("    task->joined = false;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    task->stop_requested = 0;\n");
    out.push_str("#else\n");
    out.push_str("    task->stop_requested = false;\n");
    out.push_str("    if (pthread_mutex_init(&task->stop_mutex, NULL) != 0) return false;\n");
    out.push_str("#endif\n");
    out.push_str("    SkTaskLaunch *launch = (SkTaskLaunch*)malloc(sizeof(SkTaskLaunch));\n");
    out.push_str("    if (!launch) {\n");
    out.push_str("#if !defined(_WIN32)\n");
    out.push_str("        pthread_mutex_destroy(&task->stop_mutex);\n");
    out.push_str("#endif\n");
    out.push_str("        return false;\n");
    out.push_str("    }\n");
    out.push_str("    launch->task = task;\n");
    out.push_str("    launch->entry = entry;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str(
        "    task->thread = CreateThread(NULL, 0, sk_task_platform_entry, launch, 0, NULL);\n",
    );
    out.push_str("    if (!task->thread) { free(launch); return false; }\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_create(&task->thread, NULL, sk_task_platform_entry, launch) != 0) { free(launch); pthread_mutex_destroy(&task->stop_mutex); return false; }\n");
    out.push_str("#endif\n");
    out.push_str("    task->started = true;\n");
    out.push_str("    return true;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_task_request_stop(SkTask *task) {\n");
    out.push_str("    if (!task || !task->started || task->joined) sk_task_panic(\"SC-RT-303\", \"invalid task state at stop\");\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    InterlockedExchange(&task->stop_requested, 1);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_lock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task stop synchronization failed\");\n");
    out.push_str("    task->stop_requested = true;\n");
    out.push_str("    if (pthread_mutex_unlock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task stop synchronization failed\");\n");
    out.push_str("#endif\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_task_is_stopping(void) {\n");
    out.push_str("    SkTask *task = sk_current_task;\n");
    out.push_str("    if (!task) sk_task_panic(\"SC-RT-303\", \"stopping evaluated outside task context\");\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    return InterlockedCompareExchange(&task->stop_requested, 0, 0) != 0;\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_lock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task stop synchronization failed\");\n");
    out.push_str("    bool requested = task->stop_requested;\n");
    out.push_str("    if (pthread_mutex_unlock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task stop synchronization failed\");\n");
    out.push_str("    return requested;\n");
    out.push_str("#endif\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_task_join(SkTask *task) {\n");
    out.push_str("    if (!task || !task->started || task->joined) sk_task_panic(\"SC-RT-303\", \"invalid task state at wait\");\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    if (WaitForSingleObject(task->thread, INFINITE) != WAIT_OBJECT_0) sk_task_panic(\"SC-RT-302\", \"task join failed\");\n");
    out.push_str("    CloseHandle(task->thread);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_join(task->thread, NULL) != 0) sk_task_panic(\"SC-RT-302\", \"task join failed\");\n");
    out.push_str("    if (pthread_mutex_destroy(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task stop synchronization teardown failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    task->joined = true;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_task_release_context(SkTask *task) {\n");
    out.push_str("    if (!task || !task->joined || !task->context) sk_task_panic(\"SC-RT-303\", \"invalid task state at context release\");\n");
    out.push_str("    free(task->context);\n");
    out.push_str("    task->context = NULL;\n");
    out.push_str("}\n\n");
}

fn emit_channel_runtime(out: &mut String) {
    out.push_str("typedef struct {\n");
    out.push_str("    unsigned char *buffer;\n");
    out.push_str("    size_t capacity;\n");
    out.push_str("    size_t element_size;\n");
    out.push_str("    size_t head;\n");
    out.push_str("    size_t tail;\n");
    out.push_str("    size_t count;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    CRITICAL_SECTION lock;\n");
    out.push_str("    CONDITION_VARIABLE not_empty;\n");
    out.push_str("    CONDITION_VARIABLE not_full;\n");
    out.push_str("#else\n");
    out.push_str("    pthread_mutex_t lock;\n");
    out.push_str("    pthread_cond_t not_empty;\n");
    out.push_str("    pthread_cond_t not_full;\n");
    out.push_str("#endif\n");
    out.push_str("} SkChannel;\n\n");
    out.push_str("static void sk_channel_panic(const char *code, const char *message) {\n");
    out.push_str("    fprintf(stderr, \"Runtime error: [%s] %s\\n\", code, message);\n");
    out.push_str("    exit(1);\n");
    out.push_str("}\n\n");
    out.push_str(
        "static SkChannel* sk_channel_create(int64_t capacity_value, size_t element_size) {\n",
    );
    out.push_str("    if (capacity_value <= 0 || element_size == 0 || (uint64_t)capacity_value > SIZE_MAX / element_size) sk_channel_panic(\"SC-RT-312\", \"channel capacity must be positive and fit addressable memory\");\n");
    out.push_str("    SkChannel *channel = (SkChannel*)calloc(1, sizeof(SkChannel));\n");
    out.push_str(
        "    if (!channel) sk_channel_panic(\"SC-RT-311\", \"channel allocation failed\");\n",
    );
    out.push_str("    channel->capacity = (size_t)capacity_value;\n");
    out.push_str("    channel->element_size = element_size;\n");
    out.push_str(
        "    channel->buffer = (unsigned char*)calloc(channel->capacity, element_size);\n",
    );
    out.push_str("    if (!channel->buffer) { free(channel); sk_channel_panic(\"SC-RT-311\", \"channel buffer allocation failed\"); }\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    InitializeCriticalSection(&channel->lock);\n");
    out.push_str("    InitializeConditionVariable(&channel->not_empty);\n");
    out.push_str("    InitializeConditionVariable(&channel->not_full);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_init(&channel->lock, NULL) != 0) { free(channel->buffer); free(channel); sk_channel_panic(\"SC-RT-313\", \"channel mutex initialization failed\"); }\n");
    out.push_str("    if (pthread_cond_init(&channel->not_empty, NULL) != 0) { pthread_mutex_destroy(&channel->lock); free(channel->buffer); free(channel); sk_channel_panic(\"SC-RT-313\", \"channel condition initialization failed\"); }\n");
    out.push_str("    if (pthread_cond_init(&channel->not_full, NULL) != 0) { pthread_cond_destroy(&channel->not_empty); pthread_mutex_destroy(&channel->lock); free(channel->buffer); free(channel); sk_channel_panic(\"SC-RT-313\", \"channel condition initialization failed\"); }\n");
    out.push_str("#endif\n");
    out.push_str("    return channel;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_channel_send_raw(SkChannel *channel, const void *value) {\n");
    out.push_str("    if (!channel || !value) sk_channel_panic(\"SC-RT-313\", \"invalid channel send state\");\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    EnterCriticalSection(&channel->lock);\n");
    out.push_str("    while (channel->count == channel->capacity) {\n");
    out.push_str("        if (!SleepConditionVariableCS(&channel->not_full, &channel->lock, INFINITE)) sk_channel_panic(\"SC-RT-313\", \"channel send wait failed\");\n");
    out.push_str("    }\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_lock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel send lock failed\");\n");
    out.push_str("    while (channel->count == channel->capacity) {\n");
    out.push_str("        if (pthread_cond_wait(&channel->not_full, &channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel send wait failed\");\n");
    out.push_str("    }\n");
    out.push_str("#endif\n");
    out.push_str("    memcpy(channel->buffer + (channel->tail * channel->element_size), value, channel->element_size);\n");
    out.push_str("    channel->tail = (channel->tail + 1) % channel->capacity;\n");
    out.push_str("    channel->count += 1;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    WakeConditionVariable(&channel->not_empty);\n");
    out.push_str("    LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_cond_signal(&channel->not_empty) != 0 || pthread_mutex_unlock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel send notification failed\");\n");
    out.push_str("#endif\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_channel_receive_raw(SkChannel *channel, void *out_value) {\n");
    out.push_str("    if (!channel || !out_value) sk_channel_panic(\"SC-RT-313\", \"invalid channel receive state\");\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    EnterCriticalSection(&channel->lock);\n");
    out.push_str("    while (channel->count == 0) {\n");
    out.push_str("        if (!SleepConditionVariableCS(&channel->not_empty, &channel->lock, INFINITE)) sk_channel_panic(\"SC-RT-313\", \"channel receive wait failed\");\n");
    out.push_str("    }\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_lock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel receive lock failed\");\n");
    out.push_str("    while (channel->count == 0) {\n");
    out.push_str("        if (pthread_cond_wait(&channel->not_empty, &channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel receive wait failed\");\n");
    out.push_str("    }\n");
    out.push_str("#endif\n");
    out.push_str("    memcpy(out_value, channel->buffer + (channel->head * channel->element_size), channel->element_size);\n");
    out.push_str("    channel->head = (channel->head + 1) % channel->capacity;\n");
    out.push_str("    channel->count -= 1;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    WakeConditionVariable(&channel->not_full);\n");
    out.push_str("    LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_cond_signal(&channel->not_full) != 0 || pthread_mutex_unlock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel receive notification failed\");\n");
    out.push_str("#endif\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_channel_destroy(SkChannel *channel) {\n");
    out.push_str(
        "    if (!channel) sk_channel_panic(\"SC-RT-313\", \"invalid channel destroy state\");\n",
    );
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    DeleteCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_cond_destroy(&channel->not_empty) != 0 || pthread_cond_destroy(&channel->not_full) != 0 || pthread_mutex_destroy(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel synchronization teardown failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    free(channel->buffer);\n");
    out.push_str("    free(channel);\n");
    out.push_str("}\n\n");
}

fn emit_channel_typed_wrapper(out: &mut String, skadi_type: &str) {
    let c_type = map_skadi_type_to_c(Some(skadi_type));
    let suffix = channel_type_suffix(skadi_type);
    out.push_str("static int64_t sk_channel_send_");
    out.push_str(&suffix);
    out.push_str("(SkChannel *channel, ");
    out.push_str(&c_type);
    out.push_str(" value) {\n");
    out.push_str("    sk_channel_send_raw(channel, &value);\n");
    out.push_str("    return 0;\n");
    out.push_str("}\n\n");
    out.push_str("static ");
    out.push_str(&c_type);
    out.push_str(" sk_channel_receive_");
    out.push_str(&suffix);
    out.push_str("(SkChannel *channel) {\n");
    out.push_str("    ");
    out.push_str(&c_type);
    out.push_str(" value;\n");
    out.push_str("    sk_channel_receive_raw(channel, &value);\n");
    out.push_str("    return value;\n");
    out.push_str("}\n\n");
}

fn emit_channel_typed_wrappers(out: &mut String, struct_names: &[String]) {
    const BUILTIN_CHANNEL_TYPES: [&str; 18] = [
        "i8", "i16", "i32", "i64", "Int", "u8", "u16", "u32", "u64", "f32", "f64", "Float", "bool",
        "Bool", "char", "Char", "Text", "Path",
    ];
    for skadi_type in BUILTIN_CHANNEL_TYPES {
        emit_channel_typed_wrapper(out, skadi_type);
    }
    for struct_name in struct_names {
        emit_channel_typed_wrapper(out, struct_name);
    }
}

fn emit_task_trampolines(program: &Program, entries: &HashSet<String>, out: &mut String) {
    for stmt in &program.statements {
        let Statement::FunctionDef {
            name,
            params,
            returns,
            ..
        } = stmt
        else {
            continue;
        };
        if !entries.contains(name) {
            continue;
        }
        out.push_str("typedef struct {\n");
        if params.is_empty() && returns.is_none() {
            out.push_str("    unsigned char unused;\n");
        } else {
            for (index, param) in params.iter().enumerate() {
                out.push_str("    ");
                out.push_str(&map_skadi_type_to_c(param.param_type.as_deref()));
                out.push_str(" arg_");
                out.push_str(&index.to_string());
                out.push_str(";\n");
            }
            if let Some(result_type) = returns.as_deref() {
                out.push_str("    ");
                out.push_str(&map_skadi_type_to_c(Some(result_type)));
                out.push_str(" result;\n");
            }
        }
        out.push_str("} SkTaskContext_");
        out.push_str(name);
        out.push_str(";\n\n");
        out.push_str("static void sk_task_entry_");
        out.push_str(name);
        out.push_str("(SkTask *task, void *raw_context) {\n");
        out.push_str("    (void)task;\n");
        out.push_str("    SkTaskContext_");
        out.push_str(name);
        out.push_str(" *context = (SkTaskContext_");
        out.push_str(name);
        out.push_str("*)raw_context;\n");
        out.push_str("    ");
        if returns.is_some() {
            out.push_str("context->result = ");
        }
        out.push_str(map_function_name(name));
        out.push('(');
        for (index, _) in params.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            out.push_str("context->arg_");
            out.push_str(&index.to_string());
        }
        out.push_str(");\n");
        out.push_str("}\n\n");
    }
}

fn emit_task_entry_prototypes(program: &Program, entries: &HashSet<String>, out: &mut String) {
    for stmt in &program.statements {
        let Statement::FunctionDef {
            name,
            params,
            returns,
            ..
        } = stmt
        else {
            continue;
        };
        if !entries.contains(name) {
            continue;
        }
        out.push_str(&map_skadi_type_to_c(returns.as_deref()));
        out.push(' ');
        out.push_str(map_function_name(name));
        out.push('(');
        for (index, param) in params.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            out.push_str(&map_skadi_type_to_c(param.param_type.as_deref()));
            out.push(' ');
            out.push_str(&param.name);
        }
        out.push_str(");\n");
    }
    out.push('\n');
}

pub fn transpile_program_to_c(program: &Program) -> String {
    let mut out = String::new();
    let mut codegen_state = CodegenState::default();
    let struct_names = collect_struct_names(program);
    let (needs_fs_list, needs_fs_is_dir, needs_fs_join) = program_uses_fs_runtime(program);
    let needs_list_runtime = program_uses_list_runtime(program) || needs_fs_list;
    let needs_text_runtime = program_uses_text_runtime(program);
    let needs_io_runtime = program_uses_io_runtime(program);
    let needs_args_runtime = program_uses_args_runtime(program);
    let needs_math_runtime = program_uses_math_runtime(program);
    let needs_task_runtime = statement_list_uses_task_surface(&program.statements);
    let needs_channel_runtime = statement_list_uses_deferred_task_surface(&program.statements);
    let task_entries = collect_task_entries(program);
    let needs_memory_runtime = statement_list_uses_memory_surface(&program.statements)
        || needs_list_runtime
        || needs_text_runtime
        || needs_io_runtime
        || needs_fs_list
        || needs_fs_join
        || needs_args_runtime;
    let user_main_present = has_user_main(program);
    out.push_str("#include <stdio.h>\n\n");
    if needs_list_runtime
        || needs_text_runtime
        || needs_io_runtime
        || needs_memory_runtime
        || needs_task_runtime
        || needs_channel_runtime
    {
        out.push_str("#include <stddef.h>\n");
        out.push_str("#include <stdlib.h>\n");
    }
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stdbool.h>\n\n");
    if needs_text_runtime
        || needs_fs_list
        || needs_fs_join
        || needs_io_runtime
        || needs_args_runtime
        || needs_memory_runtime
        || needs_channel_runtime
    {
        out.push_str("#include <string.h>\n\n");
    }
    if needs_math_runtime {
        out.push_str("#include <math.h>\n\n");
        emit_math_runtime(&mut out);
    }
    if needs_task_runtime || needs_channel_runtime {
        out.push_str("#if defined(_WIN32)\n");
        out.push_str("#include <windows.h>\n");
        out.push_str("#else\n");
        out.push_str("#include <pthread.h>\n");
        out.push_str("#endif\n\n");
    }
    if needs_memory_runtime || needs_task_runtime {
        emit_thread_local_support(&mut out);
    }
    if needs_task_runtime {
        emit_task_runtime(&mut out);
    }
    if needs_channel_runtime {
        emit_channel_runtime(&mut out);
    }
    if needs_fs_list || needs_fs_is_dir || needs_fs_join {
        out.push_str("#include <dirent.h>\n");
        out.push_str("#include <sys/stat.h>\n\n");
    }
    if needs_memory_runtime {
        emit_memory_runtime(&mut out);
    }
    emit_struct_declarations(program, &mut out);
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
    if needs_channel_runtime {
        emit_channel_typed_wrappers(&mut out, &struct_names);
    }

    if needs_task_runtime {
        emit_task_entry_prototypes(program, &task_entries, &mut out);
        emit_task_trampolines(program, &task_entries, &mut out);
    }

    for stmt in &program.statements {
        if let Statement::FunctionDef { .. } = stmt {
            emit_function(stmt, &mut out, &mut codegen_state);
            out.push('\n');
        }
    }
    emit_struct_methods(program, &mut out, &mut codegen_state);

    if needs_args_runtime {
        out.push_str("int main(int argc, char **argv) {\n");
    } else {
        out.push_str("int main(void) {\n");
    }
    let mut declared: HashMap<String, String> = HashMap::new();
    for stmt in &program.statements {
        if !matches!(stmt, Statement::FunctionDef { .. }) {
            emit_statement(
                stmt,
                &mut out,
                1,
                &mut declared,
                None,
                None,
                &mut codegen_state,
            );
        }
    }
    if user_main_present {
        out.push_str("    ");
        out.push_str(map_function_name("main"));
        out.push_str("();\n");
    }
    emit_top_level_cleanup(program, &mut out);
    out.push_str("    return 0;\n");
    out.push_str("}\n");

    out
}

fn emit_top_level_cleanup(program: &Program, out: &mut String) {
    for stmt in program.statements.iter().rev() {
        let Statement::VarDecl {
            name,
            value,
            declared_type,
            ..
        } = stmt
        else {
            continue;
        };

        let Some(dt) = declared_type.as_deref() else {
            continue;
        };

        if channel_elem_from_decl(dt).is_some() {
            out.push_str("    sk_channel_destroy(");
            out.push_str(name);
            out.push_str(");\n");
            continue;
        }

        if let Some(elem) = list_elem_from_decl(dt) {
            let suffix = list_meta_dynamic(elem).1;
            out.push_str("    ");
            if suffix == "text" && expression_returns_owned_text_list(value) {
                out.push_str("sk_list_text_free_owned(&");
            } else {
                out.push_str("sk_list_");
                out.push_str(&suffix);
                out.push_str("_free(&");
            }
            out.push_str(name);
            out.push_str(");\n");
            continue;
        }

        if matches!(dt, "Text" | "Path") && expression_returns_owned_text(value) {
            out.push_str("    sk_free_text((void*)");
            out.push_str(name);
            out.push_str(");\n");
        }
    }
}

fn expression_returns_owned_text(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Call { name, .. }
            if matches!(name.as_str(), "input" | "read" | "slice" | "concat" | "fs.join")
    )
}

fn expression_returns_owned_text_list(expr: &Expression) -> bool {
    matches!(expr, Expression::Call { name, .. } if name == "fs.list")
}

fn emit_default_return_tail(
    out: &mut String,
    indent: usize,
    return_type: Option<&str>,
    is_danger: bool,
) {
    let pad = "    ".repeat(indent);
    if is_danger {
        out.push_str(&pad);
        out.push_str("return 1;\n");
        return;
    }
    if let Some(ret_ty) = return_type {
        out.push_str(&pad);
        match ret_ty {
            "Text" | "Path" => out.push_str("return NULL;\n"),
            "Bool" | "bool" => out.push_str("return false;\n"),
            "Float" | "f32" | "f64" => out.push_str("return 0.0;\n"),
            "Char" | "char" => out.push_str("return '\\0';\n"),
            "Int" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => {
                out.push_str("return 0;\n")
            }
            other if other.ends_with(" List") => {
                let elem = list_elem_from_decl(other).unwrap_or("i64");
                let suffix = list_meta_dynamic(elem).1;
                out.push_str("return sk_list_");
                out.push_str(&suffix);
                out.push_str("_new();\n");
            }
            other => {
                out.push_str("return (");
                out.push_str(other);
                out.push_str("){0};\n");
            }
        }
    } else {
        out.push_str(&pad);
        out.push_str("return 0;\n");
    }
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

fn emit_struct_methods(program: &Program, out: &mut String, state: &mut CodegenState) {
    for stmt in &program.statements {
        let Statement::StructDecl { name, methods, .. } = stmt else {
            continue;
        };
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
            if method.is_danger
                && let Some(ret_ty) = method.returns.as_deref()
            {
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
            emit_block(
                &method.body,
                out,
                1,
                &mut declared,
                Some(&fn_ctx),
                None,
                state,
            );
            emit_default_return_tail(out, 1, method.returns.as_deref(), method.is_danger);
            out.push_str("}\n\n");
        }
    }
}

fn program_uses_text_runtime(program: &Program) -> bool {
    fn block_has_text(block: &BlockStatement) -> bool {
        block.statements.iter().any(statement_has_text)
    }
    fn statement_has_text(stmt: &Statement) -> bool {
        match stmt {
            Statement::VarDecl { declared_type, .. } => declared_type
                .as_deref()
                .map(|t| t == "Text")
                .unwrap_or(false),
            Statement::FunctionDef { body, .. } => block_has_text(body),
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                block_has_text(then_block)
                    || else_block
                        .as_ref()
                        .map(|b| block_has_text(b))
                        .unwrap_or(false)
            }
            Statement::WhenBlock {
                cases, else_block, ..
            } => {
                cases.iter().any(|(_, b)| block_has_text(b))
                    || else_block
                        .as_ref()
                        .map(|b| block_has_text(b))
                        .unwrap_or(false)
            }
            Statement::WhileLoop { body, .. } | Statement::LoopStatement { body, .. } => {
                block_has_text(body)
            }
            Statement::DangerAssignOnError { on_error, .. }
            | Statement::DangerCallOnError { on_error, .. }
            | Statement::ListPopOnError { on_error, .. } => block_has_text(on_error),
            Statement::PlaceIn { body, on_error, .. } => {
                block_has_text(body)
                    || on_error
                        .as_ref()
                        .map(|b| block_has_text(b))
                        .unwrap_or(false)
            }
            Statement::MemoryDecl { on_error, .. } => on_error
                .as_ref()
                .map(|b| block_has_text(b))
                .unwrap_or(false),
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => {
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
            Statement::WhenBlock {
                cases, else_block, ..
            } => {
                cases.iter().any(|(_, b)| block_has_for(b))
                    || else_block
                        .as_ref()
                        .map(|b| block_has_for(b))
                        .unwrap_or(false)
            }
            Statement::WhileLoop { body, .. } | Statement::LoopStatement { body, .. } => {
                block_has_for(body)
            }
            Statement::DangerAssignOnError { on_error, .. }
            | Statement::DangerCallOnError { on_error, .. } => block_has_for(on_error),
            Statement::PlaceIn { body, on_error, .. } => {
                block_has_for(body) || on_error.as_ref().map(|b| block_has_for(b)).unwrap_or(false)
            }
            Statement::MemoryDecl { on_error, .. } => {
                on_error.as_ref().map(|b| block_has_for(b)).unwrap_or(false)
            }
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => {
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
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
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
            Statement::ForLoop {
                condition, body, ..
            } => {
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
            Statement::WhenBlock {
                when_expression,
                cases,
                else_block,
                ..
            } => {
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
            Statement::WhileLoop {
                condition, body, ..
            } => {
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
            Statement::ListPopOnError { on_error, .. } => block_uses_fs(on_error),
            Statement::PlaceIn { body, on_error, .. } => {
                let (mut nl, mut nd, mut nj) = block_uses_fs(body);
                if let Some(on_error) = on_error {
                    let (l2, d2, j2) = block_uses_fs(on_error);
                    nl |= l2;
                    nd |= d2;
                    nj |= j2;
                }
                (nl, nd, nj)
            }
            Statement::MemoryDecl { on_error, .. } => on_error
                .as_ref()
                .map(|b| block_uses_fs(b))
                .unwrap_or((false, false, false)),
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
            let is_io = matches!(
                name.as_str(),
                "output" | "input" | "read" | "write" | "args"
            );
            is_io || args.iter().any(expression_uses_io_call)
        }
        Expression::BinaryOp { left, right, .. } => {
            expression_uses_io_call(left)
                || right
                    .as_deref()
                    .map(expression_uses_io_call)
                    .unwrap_or(false)
        }
        Expression::Index { base, index } => {
            expression_uses_io_call(base) || expression_uses_io_call(index)
        }
        Expression::ListLiteral(items) => items.iter().any(expression_uses_io_call),
        Expression::StructConstruction { fields } => {
            fields.values().any(|v| expression_uses_io_call(v))
        }
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
                || right
                    .as_deref()
                    .map(expression_uses_args_call)
                    .unwrap_or(false)
        }
        Expression::Index { base, index } => {
            expression_uses_args_call(base) || expression_uses_args_call(index)
        }
        Expression::ListLiteral(items) => items.iter().any(expression_uses_args_call),
        Expression::StructConstruction { fields } => {
            fields.values().any(|v| expression_uses_args_call(v))
        }
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
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                expression_uses_io_call(condition)
                    || then_block.statements.iter().any(stmt_uses_io)
                    || else_block
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_io))
                        .unwrap_or(false)
            }
            Statement::ForLoop {
                condition, body, ..
            } => {
                condition
                    .as_ref()
                    .map(|e| expression_uses_io_call(e))
                    .unwrap_or(false)
                    || body.statements.iter().any(stmt_uses_io)
            }
            Statement::WhenBlock {
                when_expression,
                cases,
                else_block,
                ..
            } => {
                expression_uses_io_call(when_expression)
                    || cases
                        .iter()
                        .any(|(_, b)| b.statements.iter().any(stmt_uses_io))
                    || else_block
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_io))
                        .unwrap_or(false)
            }
            Statement::WhileLoop {
                condition, body, ..
            } => expression_uses_io_call(condition) || body.statements.iter().any(stmt_uses_io),
            Statement::LoopStatement { body, .. } => body.statements.iter().any(stmt_uses_io),
            Statement::DangerAssignOnError { args, on_error, .. }
            | Statement::DangerCallOnError { args, on_error, .. } => {
                args.iter().any(expression_uses_io_call)
                    || on_error.statements.iter().any(stmt_uses_io)
            }
            Statement::ListPush { value, .. } => expression_uses_io_call(value),
            Statement::ListPopOnError { on_error, .. } => {
                on_error.statements.iter().any(stmt_uses_io)
            }
            Statement::PlaceIn { body, on_error, .. } => {
                body.statements.iter().any(stmt_uses_io)
                    || on_error
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_io))
                        .unwrap_or(false)
            }
            Statement::MemoryDecl { on_error, .. } => on_error
                .as_ref()
                .map(|b| b.statements.iter().any(stmt_uses_io))
                .unwrap_or(false),
            Statement::ReturnStatement { value, .. } => value
                .as_ref()
                .map(|v| expression_uses_io_call(v))
                .unwrap_or(false),
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => statements.iter().any(stmt_uses_io),
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
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                expression_uses_args_call(condition)
                    || then_block.statements.iter().any(stmt_uses_args)
                    || else_block
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_args))
                        .unwrap_or(false)
            }
            Statement::ForLoop {
                condition, body, ..
            } => {
                condition
                    .as_ref()
                    .map(|e| expression_uses_args_call(e))
                    .unwrap_or(false)
                    || body.statements.iter().any(stmt_uses_args)
            }
            Statement::WhenBlock {
                when_expression,
                cases,
                else_block,
                ..
            } => {
                expression_uses_args_call(when_expression)
                    || cases
                        .iter()
                        .any(|(_, b)| b.statements.iter().any(stmt_uses_args))
                    || else_block
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_args))
                        .unwrap_or(false)
            }
            Statement::WhileLoop {
                condition, body, ..
            } => expression_uses_args_call(condition) || body.statements.iter().any(stmt_uses_args),
            Statement::LoopStatement { body, .. } => body.statements.iter().any(stmt_uses_args),
            Statement::DangerAssignOnError { args, on_error, .. }
            | Statement::DangerCallOnError { args, on_error, .. } => {
                args.iter().any(expression_uses_args_call)
                    || on_error.statements.iter().any(stmt_uses_args)
            }
            Statement::ListPush { value, .. } => expression_uses_args_call(value),
            Statement::ListPopOnError { on_error, .. } => {
                on_error.statements.iter().any(stmt_uses_args)
            }
            Statement::PlaceIn { body, on_error, .. } => {
                body.statements.iter().any(stmt_uses_args)
                    || on_error
                        .as_ref()
                        .map(|b| b.statements.iter().any(stmt_uses_args))
                        .unwrap_or(false)
            }
            Statement::MemoryDecl { on_error, .. } => on_error
                .as_ref()
                .map(|b| b.statements.iter().any(stmt_uses_args))
                .unwrap_or(false),
            Statement::ReturnStatement { value, .. } => value
                .as_ref()
                .map(|v| expression_uses_args_call(v))
                .unwrap_or(false),
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => statements.iter().any(stmt_uses_args),
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

fn emit_function(stmt: &Statement, out: &mut String, state: &mut CodegenState) {
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
        out.push_str(map_function_name(name));
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
        emit_block(body, out, 1, &mut declared, Some(&fn_ctx), None, state);
        emit_default_return_tail(out, 1, returns.as_deref(), *is_danger);
        out.push_str("}\n");
    }
}

fn emit_block(
    block: &BlockStatement,
    out: &mut String,
    indent: usize,
    declared: &mut HashMap<String, String>,
    fn_ctx: Option<&FunctionContext>,
    place_ctx: Option<&PlaceContext>,
    state: &mut CodegenState,
) {
    for stmt in &block.statements {
        emit_statement(stmt, out, indent, declared, fn_ctx, place_ctx, state);
        if let Some(place_ctx) = place_ctx {
            let pad = "    ".repeat(indent);
            out.push_str(&pad);
            out.push_str("if (sk_mem_failed(");
            out.push_str(&place_ctx.memory_name);
            out.push_str(")) goto ");
            out.push_str(&place_ctx.fail_label);
            out.push_str(";\n");
        }
    }
    for stmt in block.statements.iter().rev() {
        if let Statement::VarDecl {
            name,
            declared_type: Some(declared_type),
            ..
        } = stmt
            && channel_elem_from_decl(declared_type).is_some()
        {
            out.push_str(&"    ".repeat(indent));
            out.push_str("sk_channel_destroy(");
            out.push_str(name);
            out.push_str(");\n");
        }
    }
}

fn emit_owned_channel_cleanup(out: &mut String, pad: &str, declared: &HashMap<String, String>) {
    let mut channel_names: Vec<&str> = declared
        .iter()
        .filter_map(|(name, declared_type)| {
            declared_type.ends_with("@owned").then_some(name.as_str())
        })
        .collect();
    channel_names.sort_unstable();
    for channel_name in channel_names.into_iter().rev() {
        out.push_str(pad);
        out.push_str("sk_channel_destroy(");
        out.push_str(channel_name);
        out.push_str(");\n");
    }
}

fn emit_statement(
    stmt: &Statement,
    out: &mut String,
    indent: usize,
    declared: &mut HashMap<String, String>,
    fn_ctx: Option<&FunctionContext>,
    place_ctx: Option<&PlaceContext>,
    state: &mut CodegenState,
) {
    let pad = "    ".repeat(indent);
    match stmt {
        Statement::MemoryDecl {
            name,
            size_spec,
            on_error,
            ..
        } => {
            let size_bytes = parse_memory_size_to_bytes(size_spec).unwrap_or(0);
            out.push_str(&pad);
            out.push_str("SkMemoryRegion ");
            out.push_str(name);
            out.push_str("_storage = {0};\n");
            out.push_str(&pad);
            out.push_str("SkMemoryRegion *");
            out.push_str(name);
            out.push_str(" = &");
            out.push_str(name);
            out.push_str("_storage;\n");
            out.push_str(&pad);
            out.push_str("if (!sk_mem_region_init(");
            out.push_str(name);
            out.push_str(", ");
            out.push_str(&format!("{size_bytes}ull"));
            out.push_str(")) {\n");
            if let Some(on_error) = on_error {
                let mut inner = declared.clone();
                emit_block(
                    on_error,
                    out,
                    indent + 1,
                    &mut inner,
                    fn_ctx,
                    place_ctx,
                    state,
                );
            } else {
                out.push_str(&"    ".repeat(indent + 1));
                out.push_str("sk_mem_panic(\"Memory allocation failed\");\n");
            }
            out.push_str(&pad);
            out.push_str("}\n");
            declared.insert(name.clone(), "Memory".to_string());
        }
        Statement::Assignment { target, value, .. } => {
            let expr = emit_expr(value, declared);
            out.push_str(&pad);
            out.push_str(target);
            out.push_str(" = ");
            out.push_str(&expr);
            out.push_str(";\n");
        }
        Statement::IncDec {
            target,
            is_increment,
            ..
        } => {
            out.push_str(&pad);
            out.push_str(target);
            if *is_increment {
                out.push_str(" += 1;\n");
            } else {
                out.push_str(" -= 1;\n");
            }
        }
        Statement::FieldAssignment {
            object,
            field,
            value,
            ..
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
        Statement::IfStatement {
            condition,
            then_block,
            else_block,
            ..
        } => {
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(&emit_expr(condition, declared));
            out.push_str(") {\n");
            let mut then_decl = declared.clone();
            emit_block(
                then_block,
                out,
                indent + 1,
                &mut then_decl,
                fn_ctx,
                place_ctx,
                state,
            );
            out.push_str(&pad);
            out.push('}');
            if let Some(else_block) = else_block {
                out.push_str(" else {\n");
                let mut else_decl = declared.clone();
                emit_block(
                    else_block,
                    out,
                    indent + 1,
                    &mut else_decl,
                    fn_ctx,
                    place_ctx,
                    state,
                );
                out.push_str(&pad);
                out.push('}');
            }
            out.push('\n');
        }
        Statement::WhileLoop {
            condition, body, ..
        } => {
            out.push_str(&pad);
            out.push_str("while (");
            out.push_str(&emit_expr(condition, declared));
            out.push_str(") {\n");
            let mut inner = declared.clone();
            emit_block(body, out, indent + 1, &mut inner, fn_ctx, place_ctx, state);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::LoopStatement { body, .. } => {
            out.push_str(&pad);
            out.push_str("while (1) {\n");
            let mut inner = declared.clone();
            emit_block(body, out, indent + 1, &mut inner, fn_ctx, place_ctx, state);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::BreakStatement { .. } => {
            if let Some(place_ctx) = place_ctx {
                out.push_str(&pad);
                out.push_str("sk_mem_set_active(");
                out.push_str(&place_ctx.restore_region_var);
                out.push_str(");\n");
            }
            out.push_str(&pad);
            out.push_str("break;\n");
        }
        Statement::ContinueStatement { .. } => {
            if let Some(place_ctx) = place_ctx {
                out.push_str(&pad);
                out.push_str("sk_mem_set_active(");
                out.push_str(&place_ctx.restore_region_var);
                out.push_str(");\n");
            }
            out.push_str(&pad);
            out.push_str("continue;\n");
        }
        Statement::PassStatement { .. } => {
            out.push_str(&pad);
            out.push_str(";\n");
        }
        Statement::ForLoop {
            initialization,
            condition,
            body,
            ..
        } => {
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
                    expr if is_text_list_expr(expr, declared) => "char*".to_string(),
                    _ => "int64_t".to_string(),
                };
                let item_decl_ty = match coll.as_ref() {
                    Expression::VariableReference(coll_name) => declared
                        .get(coll_name)
                        .and_then(|t| list_elem_from_decl(t))
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Int".to_string()),
                    expr if is_text_list_expr(expr, declared) => "Text".to_string(),
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
                emit_block(body, out, indent + 1, &mut inner, fn_ctx, place_ctx, state);
                out.push_str(&pad);
                out.push_str("}\n");
            } else {
                out.push_str(&pad);
                out.push_str("/* TODO(v1): unsupported for-loop form; expected 'for item in collection' */\n");
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
        Statement::DangerAssignOnError {
            target,
            call_name,
            args,
            on_error,
            ..
        } => {
            out.push_str(&pad);
            out.push_str("/* TODO(v1): danger call lowering */\n");
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(map_function_name(call_name));
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
            emit_block(
                on_error,
                out,
                indent + 1,
                &mut inner,
                fn_ctx,
                place_ctx,
                state,
            );
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
            out.push_str("/* TODO(v1): danger call lowering */\n");
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(map_function_name(call_name));
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&emit_expr(a, declared));
            }
            out.push_str(") != 0) {\n");
            let mut inner = declared.clone();
            emit_block(
                on_error,
                out,
                indent + 1,
                &mut inner,
                fn_ctx,
                place_ctx,
                state,
            );
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::ListPush {
            list_name, value, ..
        } => {
            let elem_type = declared
                .get(list_name)
                .and_then(|t| list_elem_from_decl(t))
                .map(str::to_string);
            let suffix = declared
                .get(list_name)
                .and_then(|t| list_elem_from_decl(t))
                .map(|elem| list_meta_dynamic(elem).1)
                .unwrap_or_else(|| "i64".to_string());
            out.push_str(&pad);
            out.push_str("if (sk_list_");
            out.push_str(&suffix);
            out.push_str("_push(&");
            out.push_str(list_name);
            out.push_str(", ");
            if let (Some(elem), Expression::StructConstruction { fields }) =
                (elem_type.as_deref(), value.as_ref())
            {
                out.push_str(&emit_struct_literal(fields, Some(elem), declared));
            } else {
                out.push_str(&emit_expr(value, declared));
            }
            out.push_str(") != 0) {\n");
            out.push_str(&"    ".repeat(indent + 1));
            if let Some(place_ctx) = place_ctx {
                out.push_str("goto ");
                out.push_str(&place_ctx.fail_label);
                out.push_str(";\n");
            } else {
                out.push_str("sk_mem_panic(\"list push allocation failed\");\n");
            }
            out.push_str(&pad);
            out.push_str("}\n");
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
            emit_block(
                on_error,
                out,
                indent + 1,
                &mut inner,
                fn_ctx,
                place_ctx,
                state,
            );
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::PlaceIn {
            memory_name,
            body,
            on_error,
            ..
        } => {
            let id = state.next_id();
            let previous_region = format!("__sk_prev_region_{id}");
            let fail_label = format!("__sk_place_fail_{id}");
            let end_label = format!("__sk_place_end_{id}");
            out.push_str(&pad);
            out.push_str("SkMemoryRegion *");
            out.push_str(&previous_region);
            out.push_str(" = sk_mem_set_active(");
            out.push_str(memory_name);
            out.push_str(");\n");
            out.push_str(&pad);
            out.push_str("sk_mem_clear_failure(");
            out.push_str(memory_name);
            out.push_str(");\n");
            let inner_place = PlaceContext {
                memory_name: memory_name.clone(),
                restore_region_var: previous_region.clone(),
                fail_label: fail_label.clone(),
            };
            let mut inner = declared.clone();
            emit_block(
                body,
                out,
                indent,
                &mut inner,
                fn_ctx,
                Some(&inner_place),
                state,
            );
            out.push_str(&pad);
            out.push_str("sk_mem_set_active(");
            out.push_str(&previous_region);
            out.push_str(");\n");
            out.push_str(&pad);
            out.push_str("goto ");
            out.push_str(&end_label);
            out.push_str(";\n");
            out.push_str(&pad);
            out.push_str(&fail_label);
            out.push_str(":\n");
            out.push_str(&pad);
            out.push_str("sk_mem_clear_failure(");
            out.push_str(memory_name);
            out.push_str(");\n");
            out.push_str(&pad);
            out.push_str("sk_mem_set_active(");
            out.push_str(&previous_region);
            out.push_str(");\n");
            if let Some(on_error) = on_error {
                let mut on_error_declared = declared.clone();
                emit_block(
                    on_error,
                    out,
                    indent,
                    &mut on_error_declared,
                    fn_ctx,
                    place_ctx,
                    state,
                );
            } else {
                out.push_str(&pad);
                out.push_str("sk_mem_panic(\"memory overflow in place in block\");\n");
            }
            out.push_str(&pad);
            out.push_str(&end_label);
            out.push_str(":\n");
        }
        Statement::MemoryClear { memory_name, .. } => {
            out.push_str(&pad);
            out.push_str("sk_mem_region_clear(");
            out.push_str(memory_name);
            out.push_str(");\n");
        }
        Statement::StopTask { task_name, .. } => {
            out.push_str(&pad);
            out.push_str("sk_task_request_stop(&");
            out.push_str(task_name);
            out.push_str(");\n");
        }
        Statement::ReturnStatement { value, .. } => {
            if let Some(place_ctx) = place_ctx {
                out.push_str(&pad);
                out.push_str("sk_mem_set_active(");
                out.push_str(&place_ctx.restore_region_var);
                out.push_str(");\n");
            }
            if let Some(ctx) = fn_ctx
                && ctx.is_danger
            {
                match (ctx.return_type.is_some(), value) {
                    (true, Some(expr)) => {
                        out.push_str(&pad);
                        out.push_str("*out = ");
                        out.push_str(&emit_return_expr(
                            expr,
                            ctx.return_type.as_deref(),
                            declared,
                        ));
                        out.push_str(";\n");
                        emit_owned_channel_cleanup(out, &pad, declared);
                        out.push_str(&pad);
                        out.push_str("return 0;\n");
                        return;
                    }
                    (true, None) => {
                        emit_owned_channel_cleanup(out, &pad, declared);
                        out.push_str(&pad);
                        out.push_str("return 1;\n");
                        return;
                    }
                    (false, Some(expr)) => {
                        let has_owned_channel = declared.values().any(|ty| ty.ends_with("@owned"));
                        out.push_str(&pad);
                        if has_owned_channel {
                            out.push_str("int sk_channel_return_value_");
                        } else {
                            out.push_str("return ");
                        }
                        let return_id = has_owned_channel.then(|| state.next_id());
                        if let Some(return_id) = return_id {
                            out.push_str(&return_id.to_string());
                            out.push_str(" = ");
                        }
                        out.push_str(&emit_return_expr(
                            expr,
                            ctx.return_type.as_deref(),
                            declared,
                        ));
                        out.push_str(";\n");
                        emit_owned_channel_cleanup(out, &pad, declared);
                        if let Some(return_id) = return_id {
                            out.push_str(&pad);
                            out.push_str("return sk_channel_return_value_");
                            out.push_str(&return_id.to_string());
                            out.push_str(";\n");
                        }
                        return;
                    }
                    (false, None) => {
                        emit_owned_channel_cleanup(out, &pad, declared);
                        out.push_str(&pad);
                        out.push_str("return 1;\n");
                        return;
                    }
                }
            }
            let has_owned_channel = declared.values().any(|ty| ty.ends_with("@owned"));
            let return_temp = value.as_ref().filter(|_| has_owned_channel).map(|expr| {
                let return_id = state.next_id();
                out.push_str(&pad);
                out.push_str(&map_skadi_type_to_c(
                    fn_ctx.and_then(|ctx| ctx.return_type.as_deref()),
                ));
                out.push_str(" sk_channel_return_value_");
                out.push_str(&return_id.to_string());
                out.push_str(" = ");
                out.push_str(&emit_return_expr(
                    expr,
                    fn_ctx.and_then(|ctx| ctx.return_type.as_deref()),
                    declared,
                ));
                out.push_str(";\n");
                return_id
            });
            emit_owned_channel_cleanup(out, &pad, declared);
            out.push_str(&pad);
            out.push_str("return");
            if let Some(return_id) = return_temp {
                out.push(' ');
                out.push_str("sk_channel_return_value_");
                out.push_str(&return_id.to_string());
            } else if let Some(expr) = value {
                out.push(' ');
                out.push_str(&emit_return_expr(
                    expr,
                    fn_ctx.and_then(|ctx| ctx.return_type.as_deref()),
                    declared,
                ));
            }
            out.push_str(";\n");
        }
        Statement::ExpressionStatement { expr, .. } => {
            out.push_str(&pad);
            if let Expression::WaitTask { task_name } = expr.as_ref() {
                out.push_str("sk_task_join(&");
                out.push_str(task_name);
                out.push_str(");\n");
                out.push_str(&pad);
                out.push_str("sk_task_release_context(&");
                out.push_str(task_name);
                out.push_str(");\n");
            } else {
                out.push_str(&emit_expr(expr, declared));
                out.push_str(";\n");
            }
        }
        Statement::ReturnError { code, .. } => {
            if let Some(place_ctx) = place_ctx {
                out.push_str(&pad);
                out.push_str("sk_mem_set_active(");
                out.push_str(&place_ctx.restore_region_var);
                out.push_str(");\n");
            }
            emit_owned_channel_cleanup(out, &pad, declared);
            let variant = code.rsplit('.').next().unwrap_or(code.as_str());
            out.push_str(&pad);
            out.push_str("return ErrorCode_");
            out.push_str(variant);
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
                    emit_block(else_block, out, indent, declared, fn_ctx, place_ctx, state);
                }
                return;
            }
            let when_expr = emit_expr(when_expression, declared);
            let when_tmp = format!("__when_tmp_{}", indent);
            let when_is_text = is_text_expr(when_expression, declared)
                || cases
                    .iter()
                    .any(|(case_exprs, _)| case_exprs.iter().any(|e| is_text_expr(e, declared)));
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
                emit_block(
                    case_block,
                    out,
                    indent + 1,
                    &mut inner,
                    fn_ctx,
                    place_ctx,
                    state,
                );
                out.push_str(&pad);
                out.push_str("}\n");
            }
            if let Some(else_block) = else_block {
                out.push_str(&pad);
                out.push_str("else {\n");
                let mut inner = declared.clone();
                emit_block(
                    else_block,
                    out,
                    indent + 1,
                    &mut inner,
                    fn_ctx,
                    place_ctx,
                    state,
                );
                out.push_str(&pad);
                out.push_str("}\n");
            }
        }
        Statement::VarDecl {
            name,
            value,
            declared_type,
            ..
        } => {
            if let Some(channel_element) = declared_type.as_deref().and_then(channel_elem_from_decl)
                && let Expression::Call {
                    name: call_name,
                    args,
                } = value.as_ref()
                && call_name == "channel"
            {
                out.push_str(&pad);
                out.push_str("SkChannel *");
                out.push_str(name);
                out.push_str(" = sk_channel_create(");
                out.push_str(&emit_expr(&args[0], declared));
                out.push_str(", sizeof(");
                out.push_str(&map_skadi_type_to_c(Some(channel_element)));
                out.push_str("));\n");
                declared.insert(name.clone(), format!("Channel({channel_element})@owned"));
                return;
            }
            if let Expression::WaitTask { task_name } = value.as_ref()
                && let Some((_, call_name)) = declared
                    .get(task_name)
                    .and_then(|task_type| task_type.split_once('@'))
            {
                out.push_str(&pad);
                out.push_str("sk_task_join(&");
                out.push_str(task_name);
                out.push_str(");\n");
                out.push_str(&pad);
                out.push_str(&map_skadi_type_to_c(declared_type.as_deref()));
                out.push(' ');
                out.push_str(name);
                out.push_str(" = ((SkTaskContext_");
                out.push_str(call_name);
                out.push_str("*)");
                out.push_str(task_name);
                out.push_str(".context)->result;\n");
                out.push_str(&pad);
                out.push_str("sk_task_release_context(&");
                out.push_str(task_name);
                out.push_str(");\n");
                declared.insert(
                    name.clone(),
                    declared_type.clone().unwrap_or_else(|| "Int".to_string()),
                );
                return;
            }
            if declared_type
                .as_deref()
                .map(|task_type| task_type == "Task" || task_type.starts_with("Task("))
                .unwrap_or(false)
                && let Expression::RunTask { call_name, args } = value.as_ref()
            {
                out.push_str(&pad);
                out.push_str("SkTask ");
                out.push_str(name);
                out.push_str(" = {0};\n");
                out.push_str(&pad);
                out.push_str("SkTaskContext_");
                out.push_str(call_name);
                out.push_str(" *");
                out.push_str(name);
                out.push_str("_context = (SkTaskContext_");
                out.push_str(call_name);
                out.push_str("*)malloc(sizeof(SkTaskContext_");
                out.push_str(call_name);
                out.push_str("));\n");
                out.push_str(&pad);
                out.push_str("if (!");
                out.push_str(name);
                out.push_str(
                    "_context) sk_task_panic(\"SC-RT-301\", \"task context allocation failed\");\n",
                );
                if args.is_empty() && declared_type.as_deref() == Some("Task") {
                    out.push_str(&pad);
                    out.push_str(name);
                    out.push_str("_context->unused = 0;\n");
                } else {
                    for (index, arg) in args.iter().enumerate() {
                        out.push_str(&pad);
                        out.push_str(name);
                        out.push_str("_context->arg_");
                        out.push_str(&index.to_string());
                        out.push_str(" = ");
                        out.push_str(&emit_expr(arg, declared));
                        out.push_str(";\n");
                    }
                }
                out.push_str(&pad);
                out.push_str("if (!sk_task_start(&");
                out.push_str(name);
                out.push_str(", sk_task_entry_");
                out.push_str(call_name);
                out.push_str(", ");
                out.push_str(name);
                out.push_str("_context)) { free(");
                out.push_str(name);
                out.push_str(
                    "_context); sk_task_panic(\"SC-RT-301\", \"native task creation failed\"); }\n",
                );
                declared.insert(
                    name.clone(),
                    format!(
                        "{}@{}",
                        declared_type.as_deref().unwrap_or("Task"),
                        call_name
                    ),
                );
                return;
            }
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
                        out.push_str("if (sk_list_");
                        out.push_str(&suffix);
                        out.push_str("_push(&");
                        out.push_str(name);
                        out.push_str(", ");
                        if let Expression::StructConstruction { fields } = item {
                            out.push_str(&emit_struct_literal(fields, Some(elem), declared));
                        } else {
                            out.push_str(&emit_expr(item, declared));
                        }
                        out.push_str(") != 0) {\n");
                        out.push_str(&"    ".repeat(indent + 1));
                        if let Some(place_ctx) = place_ctx {
                            out.push_str("goto ");
                            out.push_str(&place_ctx.fail_label);
                            out.push_str(";\n");
                        } else {
                            out.push_str("sk_mem_panic(\"list literal allocation failed\");\n");
                        }
                        out.push_str(&pad);
                        out.push_str("}\n");
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
            declared.insert(
                name.clone(),
                declared_type.clone().unwrap_or_else(|| "Int".to_string()),
            );
        }
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => {
            let mut inner = declared.clone();
            for s in statements {
                emit_statement(s, out, indent, &mut inner, fn_ctx, place_ctx, state);
            }
        }
    }
}

fn map_skadi_type_to_c(skadi_type: Option<&str>) -> String {
    let normalized_owned = normalize_type_token(skadi_type.unwrap_or("Int"));
    let normalized = normalized_owned.as_str();
    if channel_elem_from_decl(normalized).is_some() {
        return "SkChannel*".to_string();
    }
    if let Some(list_elem) = list_elem_from_decl(normalized) {
        let suffix = list_meta_dynamic(list_elem).1;
        return format!("SkadiList_{}", suffix);
    }
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
        "Memory" => "SkMemoryRegion*".to_string(),
        "Text" | "Path" => "const char*".to_string(),
        other => other.to_string(),
    }
}

fn parse_memory_size_to_bytes(size_spec: &str) -> Option<u64> {
    let compact = size_spec.trim().replace(' ', "").to_ascii_lowercase();
    if compact.is_empty() {
        return None;
    }
    let digits_len = compact.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_len == 0 || digits_len >= compact.len() {
        return None;
    }
    let (digits, unit) = compact.split_at(digits_len);
    let value = digits.parse::<u64>().ok()?;
    let multiplier = match unit {
        "b" => 1,
        "kb" => 1024,
        "mb" => 1024 * 1024,
        "gb" => 1024 * 1024 * 1024,
        _ => return None,
    };
    value.checked_mul(multiplier)
}

fn emit_return_expr(
    expr: &Expression,
    return_type: Option<&str>,
    declared: &HashMap<String, String>,
) -> String {
    if let (Some(ret_ty), Expression::StructConstruction { fields }) = (return_type, expr) {
        return emit_struct_literal(fields, Some(&normalize_type_token(ret_ty)), declared);
    }
    emit_expr(expr, declared)
}

fn normalize_type_token(raw: &str) -> String {
    if let Some(inner) = raw.strip_prefix("Task(").and_then(|s| s.strip_suffix(')')) {
        return format!("Task({})", normalize_type_token(inner.trim()));
    }
    if let Some(inner) = raw
        .strip_prefix("Channel(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return format!("Channel({})", normalize_type_token(inner.trim()));
    }
    if let Some(elem) = raw.strip_suffix(" List") {
        return format!("{} List", normalize_type_token(elem.trim()));
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
        Expression::MemberAccess { .. } => false,
        Expression::Call { name, .. } => matches!(
            name.as_str(),
            "input" | "read" | "slice" | "concat" | "fs.join"
        ),
        _ => false,
    }
}

fn is_text_list_expr(expr: &Expression, declared: &HashMap<String, String>) -> bool {
    match expr {
        Expression::VariableReference(name) => declared
            .get(name)
            .and_then(|t| list_elem_from_decl(t))
            .map(|elem| elem == "Text" || elem == "Path")
            .unwrap_or(false),
        Expression::Call { name, .. } => name == "fs.list",
        _ => false,
    }
}

fn expr_kind(expr: &Expression, declared: &HashMap<String, String>) -> ExprKind {
    match expr {
        Expression::LiteralInt(_) => ExprKind::Int,
        Expression::LiteralFloat(_) => ExprKind::Float,
        Expression::LiteralBool(_) => ExprKind::Bool,
        Expression::LiteralString(_) => ExprKind::Text,
        Expression::VariableReference(name) => match declared.get(name).map(String::as_str) {
            Some("Float" | "f32" | "f64") => ExprKind::Float,
            Some("bool" | "Bool") => ExprKind::Bool,
            Some("char" | "Char") => ExprKind::Char,
            Some("Text" | "Path") => ExprKind::Text,
            Some(_) => ExprKind::Int,
            None if matches!(name.as_str(), "PI" | "TAU" | "E" | "EPSILON") => ExprKind::Float,
            None => ExprKind::Unknown,
        },
        Expression::Call { name, .. } => match name.as_str() {
            "contains" | "fs.is_dir" => ExprKind::Bool,
            "find" | "len" | "write" | "output" => ExprKind::Int,
            "input" | "read" | "slice" | "concat" | "fs.join" => ExprKind::Text,
            "abs" | "min" | "max" | "clamp" | "floor" | "ceil" | "round" | "sin" | "cos"
            | "atan2" | "sqrt" | "root" | "deg_to_rad" | "rad_to_deg" => ExprKind::Float,
            _ => ExprKind::Unknown,
        },
        Expression::Index { base, .. } if is_text_expr(base, declared) => ExprKind::Char,
        Expression::BinaryOp { op, left, right } => {
            if matches!(
                op.as_str(),
                "==" | "!=" | "<" | "<=" | ">" | ">=" | "and" | "or" | "not"
            ) {
                return ExprKind::Bool;
            }
            let left_kind = expr_kind(left, declared);
            let right_kind = right
                .as_ref()
                .map(|r| expr_kind(r, declared))
                .unwrap_or(left_kind);
            if left_kind == ExprKind::Float || right_kind == ExprKind::Float {
                ExprKind::Float
            } else if left_kind == ExprKind::Int && right_kind == ExprKind::Int {
                ExprKind::Int
            } else {
                ExprKind::Unknown
            }
        }
        _ => ExprKind::Unknown,
    }
}

fn emit_expr(expr: &Expression, declared: &HashMap<String, String>) -> String {
    match expr {
        Expression::LiteralInt(v) => v.to_string(),
        Expression::LiteralFloat(v) => v.to_string(),
        Expression::LiteralBool(v) => {
            if *v {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Expression::LiteralString(s) => s.clone(),
        Expression::VariableReference(name) => match name.as_str() {
            "PI" => "M_PI".to_string(),
            "TAU" => "(2.0 * M_PI)".to_string(),
            "E" => "M_E".to_string(),
            "EPSILON" => "1e-9".to_string(),
            _ => name.clone(),
        },
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
                return format!(
                    "sk_list_{}_get(&{}, {})",
                    suffix, base_rendered, index_rendered
                );
            }
            format!("{}.data[{}]", base_rendered, index_rendered)
        }
        Expression::Call { name, args } => {
            if let Some((channel_name, method)) = name.split_once('.')
                && let Some(channel_element) = declared
                    .get(channel_name)
                    .and_then(|declared_type| channel_elem_from_decl(declared_type))
            {
                let suffix = channel_type_suffix(channel_element);
                if method == "send" && args.len() == 1 {
                    return format!(
                        "sk_channel_send_{}({}, {})",
                        suffix,
                        channel_name,
                        emit_expr(&args[0], declared)
                    );
                }
                if method == "receive" && args.is_empty() {
                    return format!("sk_channel_receive_{}({})", suffix, channel_name);
                }
            }
            if let Some(builtin) = builtin_from_name(name) {
                match builtin {
                    Builtin::Len if args.len() == 1 => {
                        let arg_rendered = emit_expr(&args[0], declared);
                        if is_text_expr(&args[0], declared)
                            || expr_kind(&args[0], declared) == ExprKind::Text
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
                        return match expr_kind(&args[0], declared) {
                            ExprKind::Float => format!("sk_output_float({})", rendered),
                            ExprKind::Bool => format!("sk_output_bool({})", rendered),
                            ExprKind::Char => format!("sk_output_char({})", rendered),
                            ExprKind::Text => format!("sk_output_text({})", rendered),
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
                    Builtin::Abs if args.len() == 1 => {
                        let a = emit_expr(&args[0], declared);
                        return match expr_kind(&args[0], declared) {
                            ExprKind::Int => format!("llabs({})", a),
                            _ => format!("fabs({})", a),
                        };
                    }
                    Builtin::Min if args.len() == 2 => {
                        let a = emit_expr(&args[0], declared);
                        let b = emit_expr(&args[1], declared);
                        return format!("(({} < {}) ? {} : {})", a, b, a, b);
                    }
                    Builtin::Max if args.len() == 2 => {
                        let a = emit_expr(&args[0], declared);
                        let b = emit_expr(&args[1], declared);
                        return format!("(({} > {}) ? {} : {})", a, b, a, b);
                    }
                    Builtin::Clamp if args.len() == 3 => {
                        let x = emit_expr(&args[0], declared);
                        let lo = emit_expr(&args[1], declared);
                        let hi = emit_expr(&args[2], declared);
                        return format!(
                            "(({} < {}) ? {} : (({} > {}) ? {} : {}))",
                            x, lo, lo, x, hi, hi, x
                        );
                    }
                    Builtin::Floor if args.len() == 1 => {
                        let a = emit_expr(&args[0], declared);
                        return format!("floor({})", a);
                    }
                    Builtin::Ceil if args.len() == 1 => {
                        let a = emit_expr(&args[0], declared);
                        return format!("ceil({})", a);
                    }
                    Builtin::Round if args.len() == 1 => {
                        let a = emit_expr(&args[0], declared);
                        return format!("round({})", a);
                    }
                    Builtin::Sin if args.len() == 1 => {
                        let a = emit_expr(&args[0], declared);
                        return format!("sin({})", a);
                    }
                    Builtin::Cos if args.len() == 1 => {
                        let a = emit_expr(&args[0], declared);
                        return format!("cos({})", a);
                    }
                    Builtin::Atan2 if args.len() == 2 => {
                        let y = emit_expr(&args[0], declared);
                        let x = emit_expr(&args[1], declared);
                        return format!("atan2({}, {})", y, x);
                    }
                    Builtin::Sqrt if args.len() == 1 => {
                        let a = emit_expr(&args[0], declared);
                        return format!("sqrt({})", a);
                    }
                    Builtin::Root if args.len() == 2 => {
                        let a = emit_expr(&args[0], declared);
                        let n = emit_expr(&args[1], declared);
                        return format!("pow({}, (1.0 / {}))", a, n);
                    }
                    Builtin::DegToRad if args.len() == 1 => {
                        let a = emit_expr(&args[0], declared);
                        return format!("(({} * M_PI) / 180.0)", a);
                    }
                    Builtin::RadToDeg if args.len() == 1 => {
                        let a = emit_expr(&args[0], declared);
                        return format!("(({} * 180.0) / M_PI)", a);
                    }
                    _ => {}
                }
            }
            if let Some((obj, method)) = name.split_once(".")
                && let Some(obj_ty) = declared.get(obj)
                && let obj_ty_norm = normalize_type_token(obj_ty)
                && !matches!(
                    obj_ty_norm.as_str(),
                    "Int"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "Float"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "Bool"
                        | "char"
                        | "Char"
                        | "Text"
                        | "Path"
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
            format!("{}({})", map_function_name(name), rendered.join(", "))
        }
        Expression::Stopping => "sk_task_is_stopping()".to_string(),
        Expression::RunTask { .. } | Expression::WaitTask { .. } => "0".to_string(),
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
                if (op == "==" || op == "!=")
                    && is_text_expr(left, declared)
                    && is_text_expr(r, declared)
                {
                    if op == "==" {
                        return format!("(strcmp({}, {}) == 0)", l, rr);
                    }
                    return format!("(strcmp({}, {}) != 0)", l, rr);
                }
                if op == "^" {
                    return format!("pow({}, {})", l, rr);
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

fn expression_uses_math_call(expr: &Expression) -> bool {
    match expr {
        Expression::VariableReference(name) => {
            matches!(name.as_str(), "PI" | "TAU" | "E" | "EPSILON")
        }
        Expression::Call { name, args } => {
            matches!(
                name.as_str(),
                "abs"
                    | "min"
                    | "max"
                    | "clamp"
                    | "floor"
                    | "ceil"
                    | "round"
                    | "sin"
                    | "cos"
                    | "atan2"
                    | "sqrt"
                    | "root"
                    | "deg_to_rad"
                    | "rad_to_deg"
            ) || args.iter().any(expression_uses_math_call)
        }
        Expression::BinaryOp { op, left, right } => {
            op == "^"
                || expression_uses_math_call(left)
                || right
                    .as_ref()
                    .map(|r| expression_uses_math_call(r))
                    .unwrap_or(false)
        }
        Expression::Index { base, index } => {
            expression_uses_math_call(base) || expression_uses_math_call(index)
        }
        Expression::ListLiteral(items) => items.iter().any(expression_uses_math_call),
        Expression::StructConstruction { fields } => {
            fields.values().any(|v| expression_uses_math_call(v))
        }
        _ => false,
    }
}

fn stmt_uses_math(stmt: &Statement) -> bool {
    match stmt {
        Statement::VarDecl { value, .. }
        | Statement::Assignment { value, .. }
        | Statement::ExpressionStatement { expr: value, .. } => expression_uses_math_call(value),
        Statement::ReturnStatement { value, .. } => value
            .as_ref()
            .map(|expr| expression_uses_math_call(expr))
            .unwrap_or(false),
        Statement::FieldAssignment { value, .. } => expression_uses_math_call(value),
        Statement::IfStatement {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_uses_math_call(condition)
                || then_block.statements.iter().any(stmt_uses_math)
                || else_block
                    .as_ref()
                    .map(|b| b.statements.iter().any(stmt_uses_math))
                    .unwrap_or(false)
        }
        Statement::WhileLoop {
            condition, body, ..
        } => expression_uses_math_call(condition) || body.statements.iter().any(stmt_uses_math),
        Statement::LoopStatement { body, .. } => body.statements.iter().any(stmt_uses_math),
        Statement::ForLoop {
            initialization,
            condition,
            update,
            body,
            ..
        } => {
            initialization
                .as_ref()
                .map(|expr| expression_uses_math_call(expr))
                .unwrap_or(false)
                || condition
                    .as_ref()
                    .map(|expr| expression_uses_math_call(expr))
                    .unwrap_or(false)
                || update
                    .as_ref()
                    .map(|expr| expression_uses_math_call(expr))
                    .unwrap_or(false)
                || body.statements.iter().any(stmt_uses_math)
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            expression_uses_math_call(when_expression)
                || cases.iter().any(|(exprs, block)| {
                    exprs.iter().any(expression_uses_math_call)
                        || block.statements.iter().any(stmt_uses_math)
                })
                || else_block
                    .as_ref()
                    .map(|b| b.statements.iter().any(stmt_uses_math))
                    .unwrap_or(false)
        }
        Statement::DangerAssignOnError { args, on_error, .. }
        | Statement::DangerCallOnError { args, on_error, .. } => {
            args.iter().any(expression_uses_math_call)
                || on_error.statements.iter().any(stmt_uses_math)
        }
        Statement::ListPush { value, .. } => expression_uses_math_call(value),
        Statement::ListPopOnError { on_error, .. } => {
            on_error.statements.iter().any(stmt_uses_math)
        }
        Statement::FunctionDef { body, .. } => body.statements.iter().any(stmt_uses_math),
        Statement::StructDecl { methods, .. } => methods
            .iter()
            .any(|m| m.body.statements.iter().any(stmt_uses_math)),
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => statements.iter().any(stmt_uses_math),
        _ => false,
    }
}

fn program_uses_math_runtime(program: &Program) -> bool {
    program.statements.iter().any(stmt_uses_math)
}
