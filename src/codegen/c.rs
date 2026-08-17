use std::collections::{HashMap, HashSet};

use crate::ast_nodes::{BlockStatement, BorrowMode, Expression, Program, Statement};
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
    function_returns: HashMap<String, String>,
    interrupt_handler_index: usize,
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

const LIST_TYPE_MAP: [(&str, &str, &str); 20] = [
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
    ("char", "char", "char"),
    ("Text", "char*", "text"),
    ("Time", "int64_t", "time"),
    ("Duration", "int64_t", "duration"),
    ("ByteSize", "int64_t", "bytesize"),
    ("Angle", "double", "angle"),
    ("Vec2", "Vec2", "vec2"),
    ("Vec3", "Vec3", "vec3"),
    ("Vec4", "Vec4", "vec4"),
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

fn type_uses_vector(raw: &str) -> bool {
    let normalized = normalize_type_token(raw);
    if matches!(normalized.as_str(), "Vec2" | "Vec3" | "Vec4") {
        return true;
    }
    normalized
        .strip_suffix(" List")
        .map(type_uses_vector)
        .or_else(|| {
            normalized
                .strip_prefix("Task(")
                .and_then(|value| value.strip_suffix(')'))
                .map(type_uses_vector)
        })
        .or_else(|| {
            normalized
                .strip_prefix("Channel(")
                .and_then(|value| value.strip_suffix(')'))
                .map(type_uses_vector)
        })
        .unwrap_or(false)
}

fn expression_uses_vector(expr: &Expression) -> bool {
    match expr {
        Expression::Call { name, args } => {
            matches!(
                name.as_str(),
                "dot" | "length" | "length_sq" | "normalize" | "distance" | "distance_sq" | "cross"
            ) || args.iter().any(expression_uses_vector)
        }
        Expression::RunTask { args, .. } | Expression::ListLiteral(args) => {
            args.iter().any(expression_uses_vector)
        }
        Expression::Index { base, index } => {
            expression_uses_vector(base) || expression_uses_vector(index)
        }
        Expression::BinaryOp { left, right, .. } => {
            expression_uses_vector(left)
                || right
                    .as_deref()
                    .map(expression_uses_vector)
                    .unwrap_or(false)
        }
        Expression::StructConstruction { fields } => {
            fields.values().any(|value| expression_uses_vector(value))
        }
        _ => false,
    }
}

fn statements_use_vector(statements: &[Statement]) -> bool {
    let block_uses_vector = |block: &BlockStatement| statements_use_vector(&block.statements);
    statements.iter().any(|statement| match statement {
        Statement::VarDecl {
            value,
            declared_type,
            ..
        } => {
            declared_type
                .as_deref()
                .map(type_uses_vector)
                .unwrap_or(false)
                || expression_uses_vector(value)
        }
        Statement::MemoryDecl { size, on_error, .. } => {
            expression_uses_vector(size)
                || on_error.as_deref().map(&block_uses_vector).unwrap_or(false)
        }
        Statement::Assignment { value, .. }
        | Statement::FieldAssignment { value, .. }
        | Statement::ListPush { value, .. }
        | Statement::ExpressionStatement { expr: value, .. } => expression_uses_vector(value),
        Statement::FunctionDef {
            params,
            returns,
            body,
            ..
        } => {
            params.iter().any(|param| {
                param
                    .param_type
                    .as_deref()
                    .map(type_uses_vector)
                    .unwrap_or(false)
            }) || returns.as_deref().map(type_uses_vector).unwrap_or(false)
                || block_uses_vector(body)
        }
        Statement::IfStatement {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_uses_vector(condition)
                || block_uses_vector(then_block)
                || else_block
                    .as_deref()
                    .map(&block_uses_vector)
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
                .map(expression_uses_vector)
                .unwrap_or(false)
                || condition
                    .as_deref()
                    .map(expression_uses_vector)
                    .unwrap_or(false)
                || update
                    .as_deref()
                    .map(expression_uses_vector)
                    .unwrap_or(false)
                || block_uses_vector(body)
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            expression_uses_vector(when_expression)
                || cases.iter().any(|(values, body)| {
                    values.iter().any(expression_uses_vector) || block_uses_vector(body)
                })
                || else_block
                    .as_deref()
                    .map(&block_uses_vector)
                    .unwrap_or(false)
        }
        Statement::WhileLoop {
            condition, body, ..
        } => expression_uses_vector(condition) || block_uses_vector(body),
        Statement::LoopStatement { body, .. } | Statement::OnBlock { body, .. } => {
            block_uses_vector(body)
        }
        Statement::StructDecl {
            fields, methods, ..
        } => {
            fields
                .iter()
                .any(|field| type_uses_vector(&field.field_type))
                || methods.iter().any(|method| {
                    method.params.iter().any(|param| {
                        param
                            .param_type
                            .as_deref()
                            .map(type_uses_vector)
                            .unwrap_or(false)
                    }) || method
                        .returns
                        .as_deref()
                        .map(type_uses_vector)
                        .unwrap_or(false)
                        || block_uses_vector(&method.body)
                })
        }
        Statement::DangerAssignOnError { args, on_error, .. }
        | Statement::DangerCallOnError { args, on_error, .. } => {
            args.iter().any(expression_uses_vector) || block_uses_vector(on_error)
        }
        Statement::ListPopOnError { on_error, .. } => block_uses_vector(on_error),
        Statement::PlaceIn { body, on_error, .. } => {
            block_uses_vector(body) || on_error.as_deref().map(&block_uses_vector).unwrap_or(false)
        }
        Statement::ReturnStatement { value, .. } => value
            .as_deref()
            .map(expression_uses_vector)
            .unwrap_or(false),
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => statements_use_vector(statements),
        Statement::IncDec { .. }
        | Statement::BreakStatement { .. }
        | Statement::ContinueStatement { .. }
        | Statement::PassStatement { .. }
        | Statement::LabelDecl { .. }
        | Statement::TagDecl { .. }
        | Statement::MemoryClear { .. }
        | Statement::StopTask { .. }
        | Statement::ReturnError { .. } => false,
    })
}

fn emit_thread_local_support(out: &mut String) {
    out.push_str("#if defined(_MSC_VER)\n");
    out.push_str("#define SK_THREAD_LOCAL __declspec(thread)\n");
    out.push_str("#else\n");
    out.push_str("#define SK_THREAD_LOCAL _Thread_local\n");
    out.push_str("#endif\n\n");
}

fn emit_memory_runtime(out: &mut String) {
    out.push_str(
        r#"typedef union {
    long double long_double_value;
    void *pointer_value;
    int64_t integer_value;
} SkMemoryAlignment;

typedef struct SkMemoryChunk {
    unsigned char *buffer;
    size_t capacity;
    size_t offset;
    bool owns_buffer;
    struct SkMemoryChunk *next;
} SkMemoryChunk;

typedef struct SkMemoryRegion {
    SkMemoryChunk first;
    SkMemoryChunk *current;
    bool failed;
    bool allow_grow;
    bool allow_drop;
} SkMemoryRegion;

typedef struct SkAllocHeader {
    uint32_t magic;
    SkMemoryRegion *owner_region;
    size_t size;
    SkMemoryAlignment alignment;
} SkAllocHeader;

#define SK_ALLOC_MAGIC 0x534B4144u

static SK_THREAD_LOCAL SkMemoryRegion *sk_active_region = NULL;

static size_t sk_mem_align_up(size_t value, size_t alignment) {
    size_t rem = value % alignment;
    return rem == 0 ? value : (value + (alignment - rem));
}

static SkAllocHeader* sk_header_from_ptr(const void *ptr) {
    if (!ptr) return NULL;
    SkAllocHeader *header = ((SkAllocHeader*)ptr) - 1;
    if (header->magic != SK_ALLOC_MAGIC) return NULL;
    return header;
}

static SkMemoryRegion* sk_mem_set_active(SkMemoryRegion *region) {
    SkMemoryRegion *previous = sk_active_region;
    sk_active_region = region;
    return previous;
}

static SkMemoryRegion* sk_mem_current(void) {
    return sk_active_region;
}

static void sk_mem_clear_failure(SkMemoryRegion *region) {
    if (region) region->failed = false;
}

static bool sk_mem_failed(SkMemoryRegion *region) {
    return region && region->failed;
}

static void sk_mem_panic(const char *message) {
    fprintf(stderr, "Skadi memory runtime error: %s\n", message ? message : "unknown");
    exit(1);
}

static bool sk_mem_region_init_external(
    SkMemoryRegion *region,
    unsigned char *buffer,
    size_t capacity,
    bool owns_buffer,
    bool allow_grow,
    bool allow_drop
) {
    if (!region || !buffer || capacity == 0) return false;
    memset(region, 0, sizeof(*region));
    region->first.buffer = buffer;
    region->first.capacity = capacity;
    region->first.owns_buffer = owns_buffer;
    region->current = &region->first;
    region->allow_grow = allow_grow;
    region->allow_drop = allow_drop;
    return true;
}

static bool sk_mem_region_init(
    SkMemoryRegion *region,
    size_t capacity,
    bool allow_grow,
    bool allow_drop
) {
    if (!region || capacity == 0) return false;
    unsigned char *buffer = (unsigned char*)malloc(capacity);
    if (!buffer) return false;
    return sk_mem_region_init_external(
        region, buffer, capacity, true, allow_grow, allow_drop
    );
}

static void sk_mem_region_release_growth(SkMemoryRegion *region) {
    if (!region) return;
    SkMemoryChunk *chunk = region->first.next;
    while (chunk) {
        SkMemoryChunk *next = chunk->next;
        if (chunk->owns_buffer) free(chunk->buffer);
        free(chunk);
        chunk = next;
    }
    region->first.next = NULL;
    region->current = &region->first;
}

static void sk_mem_region_clear(SkMemoryRegion *region) {
    if (!region) return;
    sk_mem_region_release_growth(region);
    region->first.offset = 0;
    region->failed = false;
}

static void sk_mem_region_destroy(SkMemoryRegion *region) {
    if (!region) return;
    if (sk_active_region == region) sk_active_region = NULL;
    sk_mem_region_release_growth(region);
    if (region->first.owns_buffer) free(region->first.buffer);
    memset(region, 0, sizeof(*region));
}

static SkMemoryChunk* sk_mem_grow(SkMemoryRegion *region, size_t minimum) {
    if (!region || !region->allow_grow) return NULL;
    size_t capacity = region->first.capacity;
    if (capacity < minimum) capacity = minimum;
    while (capacity < minimum || capacity < region->current->capacity * 2) {
        if (capacity > SIZE_MAX / 2) {
            capacity = minimum;
            break;
        }
        capacity *= 2;
    }
    SkMemoryChunk *chunk = (SkMemoryChunk*)calloc(1, sizeof(*chunk));
    if (!chunk) return NULL;
    chunk->buffer = (unsigned char*)malloc(capacity);
    if (!chunk->buffer) {
        free(chunk);
        return NULL;
    }
    chunk->capacity = capacity;
    chunk->owns_buffer = true;
    region->current->next = chunk;
    region->current = chunk;
    return chunk;
}

static void* sk_alloc_bytes_in(SkMemoryRegion *region, size_t size) {
    if (size > SIZE_MAX - sizeof(SkAllocHeader)) return NULL;
    size_t total = sizeof(SkAllocHeader) + size;
    if (region) {
        SkMemoryChunk *chunk = region->current;
        size_t start = sk_mem_align_up(chunk->offset, sizeof(SkMemoryAlignment));
        if (!chunk->buffer || start > chunk->capacity || total > chunk->capacity - start) {
            chunk = sk_mem_grow(region, total + sizeof(SkMemoryAlignment));
            if (!chunk) {
                region->failed = true;
                return NULL;
            }
            start = 0;
        }
        SkAllocHeader *header = (SkAllocHeader*)(chunk->buffer + start);
        header->magic = SK_ALLOC_MAGIC;
        header->owner_region = region;
        header->size = size;
        chunk->offset = start + total;
        return (void*)(header + 1);
    }
    SkAllocHeader *header = (SkAllocHeader*)malloc(total);
    if (!header) return NULL;
    header->magic = SK_ALLOC_MAGIC;
    header->owner_region = NULL;
    header->size = size;
    return (void*)(header + 1);
}

static bool sk_mem_region_init_child(
    SkMemoryRegion *child,
    SkMemoryRegion *parent,
    size_t capacity,
    bool allow_drop
) {
    if (!child || !parent || capacity == 0) return false;
    unsigned char *buffer = (unsigned char*)sk_alloc_bytes_in(parent, capacity);
    if (!buffer) return false;
    return sk_mem_region_init_external(
        child, buffer, capacity, false, false, allow_drop
    );
}

static void* sk_alloc_bytes(size_t size) {
    return sk_alloc_bytes_in(sk_mem_current(), size);
}

static char* sk_text_alloc(size_t size) {
    return (char*)sk_alloc_bytes(size + 1);
}

static char* sk_text_dup(const char *s) {
    const char *src = s ? s : "";
    size_t n = strlen(src);
    char *out = sk_text_alloc(n);
    if (!out) return NULL;
    memcpy(out, src, n);
    out[n] = '\0';
    return out;
}

static void sk_free_text(void *ptr) {
    SkAllocHeader *header = sk_header_from_ptr(ptr);
    if (!header) return;
    if (header->owner_region) return;
    free(header);
}

"#,
    );
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
    let fallback = if !matches!(c_ty, "Vec2" | "Vec3" | "Vec4")
        && LIST_TYPE_MAP
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

fn emit_list_runtime(out: &mut String, struct_names: &[String], include_vectors: bool) {
    let mut emitted_suffixes: HashSet<String> = HashSet::new();
    for (_, c_ty, suffix) in LIST_TYPE_MAP {
        if !include_vectors && matches!(c_ty, "Vec2" | "Vec3" | "Vec4") {
            continue;
        }
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

fn emit_vector_declarations(out: &mut String) {
    out.push_str("typedef struct { double x; double y; } Vec2;\n");
    out.push_str("typedef struct { double x; double y; double z; } Vec3;\n");
    out.push_str("typedef struct { double x; double y; double z; double w; } Vec4;\n\n");
}

fn emit_vector_runtime(out: &mut String, include_length_helpers: bool) {
    for (name, suffix, fields) in [
        ("Vec2", "vec2", &["x", "y"][..]),
        ("Vec3", "vec3", &["x", "y", "z"][..]),
        ("Vec4", "vec4", &["x", "y", "z", "w"][..]),
    ] {
        for (operation, symbol) in [("add", "+"), ("sub", "-")] {
            out.push_str(&format!(
                "static {name} sk_{suffix}_{operation}({name} a, {name} b) {{ return ({name}){{"
            ));
            for (index, field) in fields.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!(".{field} = a.{field} {symbol} b.{field}"));
            }
            out.push_str("}; }\n");
        }
        for (operation, symbol) in [("scale", "*"), ("div", "/")] {
            out.push_str(&format!("static {name} sk_{suffix}_{operation}({name} value, double scalar) {{ return ({name}){{"));
            for (index, field) in fields.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!(".{field} = value.{field} {symbol} scalar"));
            }
            out.push_str("}; }\n");
        }
        out.push_str(&format!(
            "static {name} sk_{suffix}_neg({name} value) {{ return ({name}){{"
        ));
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!(".{field} = -value.{field}"));
        }
        out.push_str("}; }\n");
        out.push_str(&format!(
            "static double sk_{suffix}_dot({name} a, {name} b) {{ return "
        ));
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                out.push_str(" + ");
            }
            out.push_str(&format!("a.{field} * b.{field}"));
        }
        out.push_str("; }\n");
        out.push_str(&format!("static double sk_{suffix}_length_sq({name} value) {{ return sk_{suffix}_dot(value, value); }}\n"));
        out.push_str(&format!("static double sk_{suffix}_distance_sq({name} a, {name} b) {{ return sk_{suffix}_length_sq(sk_{suffix}_sub(a, b)); }}\n"));
        if include_length_helpers {
            out.push_str(&format!("static double sk_{suffix}_length({name} value) {{ return sqrt(sk_{suffix}_length_sq(value)); }}\n"));
            out.push_str(&format!("static {name} sk_{suffix}_normalize({name} value) {{ double magnitude = sk_{suffix}_length(value); return magnitude > 0.0 ? sk_{suffix}_div(value, magnitude) : ({name}){{0}}; }}\n"));
            out.push_str(&format!("static double sk_{suffix}_distance({name} a, {name} b) {{ return sk_{suffix}_length(sk_{suffix}_sub(a, b)); }}\n"));
        }
        out.push('\n');
    }
    out.push_str("static Vec3 sk_vec3_cross(Vec3 a, Vec3 b) { return (Vec3){.x = a.y * b.z - a.z * b.y, .y = a.z * b.x - a.x * b.z, .z = a.x * b.y - a.y * b.x}; }\n\n");
}

fn emit_time_runtime(out: &mut String) {
    out.push_str("static void sk_time_panic(const char *message) {\n");
    out.push_str("    fprintf(stderr, \"Skadi time runtime error [SC-RT-320]: %s\\n\", message ? message : \"unknown\");\n");
    out.push_str("    abort();\n");
    out.push_str("}\n\n");
    out.push_str("static int64_t sk_time_now(void) {\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    LARGE_INTEGER frequency;\n");
    out.push_str("    LARGE_INTEGER counter;\n");
    out.push_str("    if (!QueryPerformanceFrequency(&frequency) || frequency.QuadPart <= 0) sk_time_panic(\"monotonic clock frequency query failed\");\n");
    out.push_str("    if (!QueryPerformanceCounter(&counter)) sk_time_panic(\"monotonic clock query failed\");\n");
    out.push_str("    return (int64_t)(((long double)counter.QuadPart * 1000000000.0L) / (long double)frequency.QuadPart);\n");
    out.push_str("#else\n");
    out.push_str("    struct timespec value;\n");
    out.push_str("    if (clock_gettime(CLOCK_MONOTONIC, &value) != 0) sk_time_panic(\"monotonic clock query failed\");\n");
    out.push_str("    return ((int64_t)value.tv_sec * 1000000000LL) + (int64_t)value.tv_nsec;\n");
    out.push_str("#endif\n");
    out.push_str("}\n\n");
    out.push_str("static int64_t sk_time_elapsed(int64_t started_at) {\n");
    out.push_str("    int64_t finished_at = sk_time_now();\n");
    out.push_str("    return finished_at >= started_at ? finished_at - started_at : 0;\n");
    out.push_str("}\n\n");
    out.push_str("static int64_t sk_time_sleep(int64_t duration_ns) {\n");
    out.push_str("    if (duration_ns <= 0) return 0;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    uint64_t remaining_ms = ((uint64_t)duration_ns + 999999ULL) / 1000000ULL;\n");
    out.push_str("    while (remaining_ms > 0) {\n");
    out.push_str("        DWORD chunk = remaining_ms > 0xFFFFFFFEULL ? 0xFFFFFFFEUL : (DWORD)remaining_ms;\n");
    out.push_str("        Sleep(chunk);\n");
    out.push_str("        remaining_ms -= chunk;\n");
    out.push_str("    }\n");
    out.push_str("#else\n");
    out.push_str("    struct timespec request;\n");
    out.push_str("    request.tv_sec = (time_t)(duration_ns / 1000000000LL);\n");
    out.push_str("    request.tv_nsec = (long)(duration_ns % 1000000000LL);\n");
    out.push_str("    while (nanosleep(&request, &request) != 0) {\n");
    out.push_str("        if (errno != EINTR) sk_time_panic(\"sleep failed\");\n");
    out.push_str("    }\n");
    out.push_str("#endif\n");
    out.push_str("    return 0;\n");
    out.push_str("}\n\n");
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
        Expression::RunTask { .. }
        | Expression::WaitTask { .. }
        | Expression::Stopping
        | Expression::TimedOut => true,
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
        | Expression::DirectBorrow(_)
        | Expression::ViewBorrow(_)
        | Expression::Move(_)
        | Expression::MemberAccess { .. }
        | Expression::LiteralInt(_)
        | Expression::LiteralFloat(_)
        | Expression::LiteralBool(_)
        | Expression::LiteralChar(_)
        | Expression::LiteralString(_)
        | Expression::LiteralDuration { .. }
        | Expression::LiteralByteSize { .. }
        | Expression::LiteralAngle { .. } => false,
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
        | Statement::LabelDecl { .. }
        | Statement::TagDecl { .. } => false,
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
                || name.ends_with(".send_for")
                || name.ends_with(".receive_for")
                || args.iter().any(expression_uses_deferred_task_surface)
        }
        Expression::RunTask { args, .. } => args.iter().any(expression_uses_deferred_task_surface),
        Expression::WaitTask { .. } | Expression::Stopping | Expression::TimedOut => false,
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
    out.push_str("typedef void (*SkTaskWake)(void *context);\n\n");
    out.push_str("struct SkTask {\n");
    out.push_str("    SkPlatformThread thread;\n");
    out.push_str("    void *context;\n");
    out.push_str("    bool started;\n");
    out.push_str("    bool joined;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    CRITICAL_SECTION stop_lock;\n");
    out.push_str("    volatile LONG stop_requested;\n");
    out.push_str("#else\n");
    out.push_str("    pthread_mutex_t stop_mutex;\n");
    out.push_str("    bool stop_requested;\n");
    out.push_str("#endif\n");
    out.push_str("    void *wait_context;\n");
    out.push_str("    SkTaskWake wake_wait;\n");
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
    out.push_str("    task->wait_context = NULL;\n");
    out.push_str("    task->wake_wait = NULL;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    InitializeCriticalSection(&task->stop_lock);\n");
    out.push_str("    task->stop_requested = 0;\n");
    out.push_str("#else\n");
    out.push_str("    task->stop_requested = false;\n");
    out.push_str("    if (pthread_mutex_init(&task->stop_mutex, NULL) != 0) return false;\n");
    out.push_str("#endif\n");
    out.push_str("    SkTaskLaunch *launch = (SkTaskLaunch*)malloc(sizeof(SkTaskLaunch));\n");
    out.push_str("    if (!launch) {\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("        DeleteCriticalSection(&task->stop_lock);\n");
    out.push_str("#else\n");
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
    out.push_str("    if (!task->thread) { free(launch); DeleteCriticalSection(&task->stop_lock); return false; }\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_create(&task->thread, NULL, sk_task_platform_entry, launch) != 0) { free(launch); pthread_mutex_destroy(&task->stop_mutex); return false; }\n");
    out.push_str("#endif\n");
    out.push_str("    task->started = true;\n");
    out.push_str("    return true;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_task_request_stop(SkTask *task) {\n");
    out.push_str("    if (!task || !task->started || task->joined) sk_task_panic(\"SC-RT-303\", \"invalid task state at stop\");\n");
    out.push_str("    void *wait_context = NULL;\n");
    out.push_str("    SkTaskWake wake_wait = NULL;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    EnterCriticalSection(&task->stop_lock);\n");
    out.push_str("    InterlockedExchange(&task->stop_requested, 1);\n");
    out.push_str("    wait_context = task->wait_context;\n");
    out.push_str("    wake_wait = task->wake_wait;\n");
    out.push_str("    LeaveCriticalSection(&task->stop_lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_lock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task stop synchronization failed\");\n");
    out.push_str("    task->stop_requested = true;\n");
    out.push_str("    wait_context = task->wait_context;\n");
    out.push_str("    wake_wait = task->wake_wait;\n");
    out.push_str("    if (pthread_mutex_unlock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task stop synchronization failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    if (wake_wait && wait_context) wake_wait(wait_context);\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_task_register_wait(void *context, SkTaskWake wake_wait) {\n");
    out.push_str("    SkTask *task = sk_current_task;\n");
    out.push_str("    if (!task) return true;\n");
    out.push_str("    bool registered = false;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    EnterCriticalSection(&task->stop_lock);\n");
    out.push_str("    if (InterlockedCompareExchange(&task->stop_requested, 0, 0) == 0) { task->wait_context = context; task->wake_wait = wake_wait; registered = true; }\n");
    out.push_str("    LeaveCriticalSection(&task->stop_lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_lock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task wait registration failed\");\n");
    out.push_str("    if (!task->stop_requested) { task->wait_context = context; task->wake_wait = wake_wait; registered = true; }\n");
    out.push_str("    if (pthread_mutex_unlock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task wait registration failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    return registered;\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_task_finish_wait(void *context) {\n");
    out.push_str("    SkTask *task = sk_current_task;\n");
    out.push_str("    if (!task) return true;\n");
    out.push_str("    bool active = true;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    EnterCriticalSection(&task->stop_lock);\n");
    out.push_str("    active = InterlockedCompareExchange(&task->stop_requested, 0, 0) == 0;\n");
    out.push_str("    if (task->wait_context == context) { task->wait_context = NULL; task->wake_wait = NULL; }\n");
    out.push_str("    LeaveCriticalSection(&task->stop_lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_lock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task wait completion failed\");\n");
    out.push_str("    active = !task->stop_requested;\n");
    out.push_str("    if (task->wait_context == context) { task->wait_context = NULL; task->wake_wait = NULL; }\n");
    out.push_str("    if (pthread_mutex_unlock(&task->stop_mutex) != 0) sk_task_panic(\"SC-RT-304\", \"task wait completion failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    return active;\n");
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
    out.push_str("    DeleteCriticalSection(&task->stop_lock);\n");
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
    out.push_str("typedef enum {\n");
    out.push_str("    SK_CHANNEL_OK = 0,\n");
    out.push_str("    SK_CHANNEL_CLOSED = 1,\n");
    out.push_str("    SK_CHANNEL_CANCELLED = 2,\n");
    out.push_str("    SK_CHANNEL_TIMED_OUT = 3\n");
    out.push_str("} SkChannelStatus;\n\n");
    out.push_str(
        "static SK_THREAD_LOCAL SkChannelStatus sk_channel_last_status = SK_CHANNEL_OK;\n\n",
    );
    out.push_str("typedef struct SkChannel {\n");
    out.push_str("    unsigned char *buffer;\n");
    out.push_str("    size_t capacity;\n");
    out.push_str("    size_t element_size;\n");
    out.push_str("    size_t head;\n");
    out.push_str("    size_t tail;\n");
    out.push_str("    size_t count;\n");
    out.push_str("    bool closed;\n");
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
    out.push_str("static bool sk_channel_timed_out(void) {\n");
    out.push_str("    return sk_channel_last_status == SK_CHANNEL_TIMED_OUT;\n");
    out.push_str("}\n\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("static DWORD sk_channel_remaining_millis(ULONGLONG deadline) {\n");
    out.push_str("    ULONGLONG now = GetTickCount64();\n");
    out.push_str("    if (now >= deadline) return 0;\n");
    out.push_str("    ULONGLONG remaining = deadline - now;\n");
    out.push_str(
        "    return remaining >= (ULONGLONG)(INFINITE - 1) ? INFINITE - 1 : (DWORD)remaining;\n",
    );
    out.push_str("}\n");
    out.push_str("#else\n");
    out.push_str("static struct timespec sk_channel_deadline(int64_t timeout_ns) {\n");
    out.push_str("    struct timespec deadline;\n");
    out.push_str("    if (clock_gettime(CLOCK_REALTIME, &deadline) != 0) sk_channel_panic(\"SC-RT-313\", \"channel clock read failed\");\n");
    out.push_str("    deadline.tv_sec += (time_t)(timeout_ns / 1000000000LL);\n");
    out.push_str("    deadline.tv_nsec += (long)(timeout_ns % 1000000000LL);\n");
    out.push_str("    if (deadline.tv_nsec >= 1000000000L) { deadline.tv_sec += 1; deadline.tv_nsec -= 1000000000L; }\n");
    out.push_str("    return deadline;\n");
    out.push_str("}\n");
    out.push_str("#endif\n\n");
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
    out.push_str("static void sk_channel_wake_waiters(void *context) {\n");
    out.push_str("    SkChannel *channel = (SkChannel*)context;\n");
    out.push_str("    if (!channel) return;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    EnterCriticalSection(&channel->lock);\n");
    out.push_str("    WakeAllConditionVariable(&channel->not_empty);\n");
    out.push_str("    WakeAllConditionVariable(&channel->not_full);\n");
    out.push_str("    LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_lock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel cancellation lock failed\");\n");
    out.push_str("    pthread_cond_broadcast(&channel->not_empty);\n");
    out.push_str("    pthread_cond_broadcast(&channel->not_full);\n");
    out.push_str("    if (pthread_mutex_unlock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel cancellation unlock failed\");\n");
    out.push_str("#endif\n");
    out.push_str("}\n\n");
    out.push_str(
        "static SkChannelStatus sk_channel_send_raw(SkChannel *channel, const void *value, int64_t timeout_ns) {\n",
    );
    out.push_str("    if (!channel || !value) sk_channel_panic(\"SC-RT-313\", \"invalid channel send state\");\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    ULONGLONG deadline = 0;\n");
    out.push_str("    if (timeout_ns >= 0) { ULONGLONG millis = (ULONGLONG)(timeout_ns / 1000000LL) + (timeout_ns % 1000000LL != 0); deadline = GetTickCount64() + millis; }\n");
    out.push_str("    EnterCriticalSection(&channel->lock);\n");
    out.push_str("    while (channel->count == channel->capacity && !channel->closed) {\n");
    out.push_str("        if (timeout_ns == 0) { LeaveCriticalSection(&channel->lock); return SK_CHANNEL_TIMED_OUT; }\n");
    out.push_str("        if (!sk_task_register_wait(channel, sk_channel_wake_waiters)) { LeaveCriticalSection(&channel->lock); return SK_CHANNEL_CANCELLED; }\n");
    out.push_str("        DWORD wait_ms = timeout_ns < 0 ? INFINITE : sk_channel_remaining_millis(deadline);\n");
    out.push_str("        BOOL woke = SleepConditionVariableCS(&channel->not_full, &channel->lock, wait_ms);\n");
    out.push_str("        DWORD wait_error = woke ? ERROR_SUCCESS : GetLastError();\n");
    out.push_str("        if (!sk_task_finish_wait(channel)) { LeaveCriticalSection(&channel->lock); return SK_CHANNEL_CANCELLED; }\n");
    out.push_str("        if (!woke && wait_error != ERROR_TIMEOUT) sk_channel_panic(\"SC-RT-313\", \"channel send wait failed\");\n");
    out.push_str("        if (!woke && wait_error == ERROR_TIMEOUT && sk_channel_remaining_millis(deadline) == 0 && channel->count == channel->capacity && !channel->closed) { LeaveCriticalSection(&channel->lock); return SK_CHANNEL_TIMED_OUT; }\n");
    out.push_str("    }\n");
    out.push_str("#else\n");
    out.push_str("    struct timespec deadline = {0};\n");
    out.push_str("    if (timeout_ns >= 0) deadline = sk_channel_deadline(timeout_ns);\n");
    out.push_str("    if (pthread_mutex_lock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel send lock failed\");\n");
    out.push_str("    while (channel->count == channel->capacity && !channel->closed) {\n");
    out.push_str("        if (timeout_ns == 0) { pthread_mutex_unlock(&channel->lock); return SK_CHANNEL_TIMED_OUT; }\n");
    out.push_str("        if (!sk_task_register_wait(channel, sk_channel_wake_waiters)) { pthread_mutex_unlock(&channel->lock); return SK_CHANNEL_CANCELLED; }\n");
    out.push_str("        int wait_result = timeout_ns < 0 ? pthread_cond_wait(&channel->not_full, &channel->lock) : pthread_cond_timedwait(&channel->not_full, &channel->lock, &deadline);\n");
    out.push_str("        if (!sk_task_finish_wait(channel)) { pthread_mutex_unlock(&channel->lock); return SK_CHANNEL_CANCELLED; }\n");
    out.push_str("        if (wait_result == ETIMEDOUT && channel->count == channel->capacity && !channel->closed) { pthread_mutex_unlock(&channel->lock); return SK_CHANNEL_TIMED_OUT; }\n");
    out.push_str("        if (wait_result != 0 && wait_result != ETIMEDOUT) sk_channel_panic(\"SC-RT-313\", \"channel send wait failed\");\n");
    out.push_str("    }\n");
    out.push_str("#endif\n");
    out.push_str("    if (channel->closed) {\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("        LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("        pthread_mutex_unlock(&channel->lock);\n");
    out.push_str("#endif\n");
    out.push_str("        return SK_CHANNEL_CLOSED;\n");
    out.push_str("    }\n");
    out.push_str("    memcpy(channel->buffer + (channel->tail * channel->element_size), value, channel->element_size);\n");
    out.push_str("    channel->tail = (channel->tail + 1) % channel->capacity;\n");
    out.push_str("    channel->count += 1;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    WakeConditionVariable(&channel->not_empty);\n");
    out.push_str("    LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_cond_signal(&channel->not_empty) != 0 || pthread_mutex_unlock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel send notification failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    return SK_CHANNEL_OK;\n");
    out.push_str("}\n\n");
    out.push_str(
        "static SkChannelStatus sk_channel_receive_raw(SkChannel *channel, void *out_value, int64_t timeout_ns) {\n",
    );
    out.push_str("    if (!channel || !out_value) sk_channel_panic(\"SC-RT-313\", \"invalid channel receive state\");\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    ULONGLONG deadline = 0;\n");
    out.push_str("    if (timeout_ns >= 0) { ULONGLONG millis = (ULONGLONG)(timeout_ns / 1000000LL) + (timeout_ns % 1000000LL != 0); deadline = GetTickCount64() + millis; }\n");
    out.push_str("    EnterCriticalSection(&channel->lock);\n");
    out.push_str("    while (channel->count == 0 && !channel->closed) {\n");
    out.push_str("        if (timeout_ns == 0) { LeaveCriticalSection(&channel->lock); return SK_CHANNEL_TIMED_OUT; }\n");
    out.push_str("        if (!sk_task_register_wait(channel, sk_channel_wake_waiters)) { LeaveCriticalSection(&channel->lock); return SK_CHANNEL_CANCELLED; }\n");
    out.push_str("        DWORD wait_ms = timeout_ns < 0 ? INFINITE : sk_channel_remaining_millis(deadline);\n");
    out.push_str("        BOOL woke = SleepConditionVariableCS(&channel->not_empty, &channel->lock, wait_ms);\n");
    out.push_str("        DWORD wait_error = woke ? ERROR_SUCCESS : GetLastError();\n");
    out.push_str("        if (!sk_task_finish_wait(channel)) { LeaveCriticalSection(&channel->lock); return SK_CHANNEL_CANCELLED; }\n");
    out.push_str("        if (!woke && wait_error != ERROR_TIMEOUT) sk_channel_panic(\"SC-RT-313\", \"channel receive wait failed\");\n");
    out.push_str("        if (!woke && wait_error == ERROR_TIMEOUT && sk_channel_remaining_millis(deadline) == 0 && channel->count == 0 && !channel->closed) { LeaveCriticalSection(&channel->lock); return SK_CHANNEL_TIMED_OUT; }\n");
    out.push_str("    }\n");
    out.push_str("#else\n");
    out.push_str("    struct timespec deadline = {0};\n");
    out.push_str("    if (timeout_ns >= 0) deadline = sk_channel_deadline(timeout_ns);\n");
    out.push_str("    if (pthread_mutex_lock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel receive lock failed\");\n");
    out.push_str("    while (channel->count == 0 && !channel->closed) {\n");
    out.push_str("        if (timeout_ns == 0) { pthread_mutex_unlock(&channel->lock); return SK_CHANNEL_TIMED_OUT; }\n");
    out.push_str("        if (!sk_task_register_wait(channel, sk_channel_wake_waiters)) { pthread_mutex_unlock(&channel->lock); return SK_CHANNEL_CANCELLED; }\n");
    out.push_str("        int wait_result = timeout_ns < 0 ? pthread_cond_wait(&channel->not_empty, &channel->lock) : pthread_cond_timedwait(&channel->not_empty, &channel->lock, &deadline);\n");
    out.push_str("        if (!sk_task_finish_wait(channel)) { pthread_mutex_unlock(&channel->lock); return SK_CHANNEL_CANCELLED; }\n");
    out.push_str("        if (wait_result == ETIMEDOUT && channel->count == 0 && !channel->closed) { pthread_mutex_unlock(&channel->lock); return SK_CHANNEL_TIMED_OUT; }\n");
    out.push_str("        if (wait_result != 0 && wait_result != ETIMEDOUT) sk_channel_panic(\"SC-RT-313\", \"channel receive wait failed\");\n");
    out.push_str("    }\n");
    out.push_str("#endif\n");
    out.push_str("    if (channel->count == 0 && channel->closed) {\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("        LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("        pthread_mutex_unlock(&channel->lock);\n");
    out.push_str("#endif\n");
    out.push_str("        return SK_CHANNEL_CLOSED;\n");
    out.push_str("    }\n");
    out.push_str("    memcpy(out_value, channel->buffer + (channel->head * channel->element_size), channel->element_size);\n");
    out.push_str("    channel->head = (channel->head + 1) % channel->capacity;\n");
    out.push_str("    channel->count -= 1;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    WakeConditionVariable(&channel->not_full);\n");
    out.push_str("    LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_cond_signal(&channel->not_full) != 0 || pthread_mutex_unlock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel receive notification failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    return SK_CHANNEL_OK;\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_channel_try_send_raw(SkChannel *channel, const void *value) {\n");
    out.push_str("    if (!channel || !value) return false;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    if (!TryEnterCriticalSection(&channel->lock)) return false;\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_trylock(&channel->lock) != 0) return false;\n");
    out.push_str("#endif\n");
    out.push_str("    if (channel->closed || channel->count == channel->capacity) {\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("        LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("        pthread_mutex_unlock(&channel->lock);\n");
    out.push_str("#endif\n");
    out.push_str("        return false;\n");
    out.push_str("    }\n");
    out.push_str("    memcpy(channel->buffer + (channel->tail * channel->element_size), value, channel->element_size);\n");
    out.push_str("    channel->tail = (channel->tail + 1) % channel->capacity;\n");
    out.push_str("    channel->count += 1;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    WakeConditionVariable(&channel->not_empty);\n");
    out.push_str("    LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("    pthread_cond_signal(&channel->not_empty);\n");
    out.push_str("    pthread_mutex_unlock(&channel->lock);\n");
    out.push_str("#endif\n");
    out.push_str("    return true;\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_channel_close(SkChannel *channel) {\n");
    out.push_str("    if (!channel) return false;\n");
    out.push_str("    bool changed = false;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    EnterCriticalSection(&channel->lock);\n");
    out.push_str("    changed = !channel->closed;\n");
    out.push_str("    if (changed) {\n");
    out.push_str("        channel->closed = true;\n");
    out.push_str("        WakeAllConditionVariable(&channel->not_empty);\n");
    out.push_str("        WakeAllConditionVariable(&channel->not_full);\n");
    out.push_str("    }\n");
    out.push_str("    LeaveCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_mutex_lock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel close lock failed\");\n");
    out.push_str("    changed = !channel->closed;\n");
    out.push_str("    if (changed) {\n");
    out.push_str("        channel->closed = true;\n");
    out.push_str("        pthread_cond_broadcast(&channel->not_empty);\n");
    out.push_str("        pthread_cond_broadcast(&channel->not_full);\n");
    out.push_str("    }\n");
    out.push_str("    if (pthread_mutex_unlock(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel close unlock failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    return changed;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_channel_destroy(SkChannel *channel) {\n");
    out.push_str("    if (!channel) return;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    DeleteCriticalSection(&channel->lock);\n");
    out.push_str("#else\n");
    out.push_str("    if (pthread_cond_destroy(&channel->not_empty) != 0 || pthread_cond_destroy(&channel->not_full) != 0 || pthread_mutex_destroy(&channel->lock) != 0) sk_channel_panic(\"SC-RT-313\", \"channel synchronization teardown failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    free(channel->buffer);\n");
    out.push_str("    free(channel);\n");
    out.push_str("}\n\n");
    out.push_str("static SkChannel* sk_channel_move(SkChannel **source) {\n");
    out.push_str("    if (!source) return NULL;\n");
    out.push_str("    SkChannel *result = *source;\n");
    out.push_str("    *source = NULL;\n");
    out.push_str("    return result;\n");
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
    out.push_str("    return (int64_t)sk_channel_send_raw(channel, &value, -1);\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_channel_send_or_panic_");
    out.push_str(&suffix);
    out.push_str("(SkChannel *channel, ");
    out.push_str(&c_type);
    out.push_str(" value) {\n");
    out.push_str("    SkChannelStatus status = sk_channel_send_raw(channel, &value, -1);\n");
    out.push_str("    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic(\"SC-RT-315\", \"cancelled channel send requires 'on error'\");\n");
    out.push_str("    if (status == SK_CHANNEL_CLOSED) sk_channel_panic(\"SC-RT-314\", \"send on closed channel requires 'on error'\");\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_channel_try_send_");
    out.push_str(&suffix);
    out.push_str("(SkChannel *channel, ");
    out.push_str(&c_type);
    out.push_str(" value) {\n");
    out.push_str("    return sk_channel_try_send_raw(channel, &value);\n");
    out.push_str("}\n\n");
    out.push_str("static int64_t sk_channel_send_for_");
    out.push_str(&suffix);
    out.push_str("(SkChannel *channel, ");
    out.push_str(&c_type);
    out.push_str(" value, int64_t timeout_ns) {\n");
    out.push_str("    return (int64_t)sk_channel_send_raw(channel, &value, timeout_ns < 0 ? 0 : timeout_ns);\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_channel_try_receive_");
    out.push_str(&suffix);
    out.push_str("(SkChannel *channel, ");
    out.push_str(&c_type);
    out.push_str(" *out) {\n");
    out.push_str("    return sk_channel_receive_raw(channel, out, -1) == SK_CHANNEL_OK;\n");
    out.push_str("}\n\n");
    out.push_str("static int64_t sk_channel_receive_for_");
    out.push_str(&suffix);
    out.push_str("(SkChannel *channel, ");
    out.push_str(&c_type);
    out.push_str(" *out, int64_t timeout_ns) {\n");
    out.push_str("    return (int64_t)sk_channel_receive_raw(channel, out, timeout_ns < 0 ? 0 : timeout_ns);\n");
    out.push_str("}\n\n");
    out.push_str("static ");
    out.push_str(&c_type);
    out.push_str(" sk_channel_receive_");
    out.push_str(&suffix);
    out.push_str("(SkChannel *channel) {\n");
    out.push_str("    ");
    out.push_str(&c_type);
    out.push_str(" value;\n");
    out.push_str("    SkChannelStatus status = sk_channel_receive_raw(channel, &value, -1);\n");
    out.push_str("    if (status == SK_CHANNEL_CANCELLED) sk_channel_panic(\"SC-RT-315\", \"cancelled channel receive requires 'on error'\");\n");
    out.push_str("    if (status == SK_CHANNEL_CLOSED) sk_channel_panic(\"SC-RT-314\", \"receive on drained closed channel requires 'on error'\");\n");
    out.push_str("    return value;\n");
    out.push_str("}\n\n");
}

fn emit_channel_typed_wrappers(out: &mut String, struct_names: &[String], include_vectors: bool) {
    const BUILTIN_CHANNEL_TYPES: [&str; 25] = [
        "i8", "i16", "i32", "i64", "Int", "u8", "u16", "u32", "u64", "f32", "f64", "Float", "bool",
        "Bool", "char", "Char", "Text", "Path", "Time", "Duration", "ByteSize", "Angle", "Vec2",
        "Vec3", "Vec4",
    ];
    for skadi_type in BUILTIN_CHANNEL_TYPES {
        if !include_vectors && matches!(skadi_type, "Vec2" | "Vec3" | "Vec4") {
            continue;
        }
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
    let mut codegen_state = CodegenState {
        function_returns: program
            .statements
            .iter()
            .filter_map(|stmt| match stmt {
                Statement::FunctionDef {
                    name,
                    returns: Some(return_type),
                    ..
                } => Some((name.clone(), return_type.clone())),
                _ => None,
            })
            .collect(),
        ..CodegenState::default()
    };
    let struct_names = collect_struct_names(program);
    let (needs_fs_list, needs_fs_is_dir, needs_fs_join) = program_uses_fs_runtime(program);
    let needs_list_runtime = program_uses_list_runtime(program) || needs_fs_list;
    let needs_text_runtime = program_uses_text_runtime(program);
    let needs_io_runtime = program_uses_io_runtime(program);
    let needs_args_runtime = program_uses_args_runtime(program);
    let needs_math_runtime = program_uses_math_runtime(program);
    let needs_visual_runtime = program_uses_visual_runtime(program);
    let needs_window_runtime = program_uses_window_runtime(program);
    let needs_vector_runtime = statements_use_vector(&program.statements) || needs_visual_runtime;
    let needs_time_runtime = program_uses_time_runtime(program);
    let needs_interrupt_runtime = program_uses_interrupt_runtime(program);
    let needs_channel_runtime = statement_list_uses_deferred_task_surface(&program.statements);
    let needs_task_runtime =
        statement_list_uses_task_surface(&program.statements) || needs_channel_runtime;
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
        || needs_time_runtime
        || needs_interrupt_runtime
        || needs_visual_runtime
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
        || needs_visual_runtime
    {
        out.push_str("#include <string.h>\n\n");
    }
    if needs_math_runtime {
        out.push_str("#include <math.h>\n\n");
        emit_math_runtime(&mut out);
    }
    if needs_task_runtime || needs_channel_runtime || needs_time_runtime || needs_interrupt_runtime
    {
        out.push_str("#if defined(_WIN32)\n");
        out.push_str("#include <windows.h>\n");
        out.push_str("#else\n");
        if needs_task_runtime || needs_channel_runtime || needs_interrupt_runtime {
            out.push_str("#include <pthread.h>\n");
        }
        if needs_channel_runtime || needs_time_runtime || needs_interrupt_runtime {
            out.push_str("#include <errno.h>\n");
            out.push_str("#include <time.h>\n");
        }
        out.push_str("#endif\n\n");
    }
    if needs_window_runtime
        && !(needs_task_runtime
            || needs_channel_runtime
            || needs_time_runtime
            || needs_interrupt_runtime)
    {
        out.push_str("#if defined(_WIN32)\n#include <windows.h>\n#endif\n\n");
    }
    if needs_vector_runtime {
        emit_vector_declarations(&mut out);
        emit_vector_runtime(&mut out, needs_math_runtime);
    }
    if needs_visual_runtime {
        emit_visual_runtime(&mut out, needs_window_runtime);
    }
    if needs_time_runtime {
        emit_time_runtime(&mut out);
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
    if needs_interrupt_runtime {
        emit_interrupt_runtime(&mut out);
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
        emit_list_runtime(&mut out, &struct_names, needs_vector_runtime);
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
    emit_nominal_enums(program, &mut out);
    if needs_channel_runtime {
        emit_channel_typed_wrappers(&mut out, &struct_names, needs_vector_runtime);
    }
    if needs_interrupt_runtime {
        emit_interrupt_handlers(program, &mut out, &mut codegen_state);
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
    codegen_state.interrupt_handler_index = 0;

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
        if let Statement::VarDecl {
            name,
            declared_type: Some(declared_type),
            ..
        } = stmt
            && normalize_type_token(declared_type) == "Interrupt"
        {
            out.push_str("    sk_interrupt_destroy(");
            out.push_str(name);
            out.push_str(");\n");
        }
    }
    for resource_type in ["Window", "Canvas"] {
        for stmt in program.statements.iter().rev() {
            if let Statement::VarDecl {
                name,
                declared_type: Some(declared_type),
                ..
            } = stmt
                && normalize_type_token(declared_type) == resource_type
            {
                out.push_str("    ");
                out.push_str(if resource_type == "Window" {
                    "sk_window_destroy(&"
                } else {
                    "sk_canvas_destroy(&"
                });
                out.push_str(name);
                out.push_str(");\n");
            }
        }
    }
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
    for stmt in program.statements.iter().rev() {
        if let Statement::MemoryDecl { name, .. } = stmt {
            out.push_str("    sk_mem_region_destroy(");
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
                let c_type = map_skadi_type_to_c(Some(other));
                out.push_str("return (");
                out.push_str(&c_type);
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

fn emit_nominal_enums(program: &Program, out: &mut String) {
    for stmt in &program.statements {
        match stmt {
            Statement::LabelDecl { name, variants, .. } if !variants.is_empty() => {
                out.push_str("typedef enum ");
                out.push_str(name);
                out.push_str(" {\n");
                for variant in variants {
                    out.push_str(&format!(
                        "    {}_{} = {},\n",
                        name, variant.name, variant.discriminant
                    ));
                }
                out.push_str("} ");
                out.push_str(name);
                out.push_str(";\n\n");
            }
            Statement::TagDecl { name, variants, .. } if !variants.is_empty() => {
                out.push_str("typedef enum ");
                out.push_str(name);
                out.push_str(" {\n");
                for (index, variant) in variants.iter().enumerate() {
                    out.push_str(&format!("    {}_{} = {},\n", name, variant, index));
                }
                out.push_str("} ");
                out.push_str(name);
                out.push_str(";\n\n");
            }
            _ => {}
        }
    }
}

fn emit_interrupt_runtime(out: &mut String) {
    out.push_str("typedef void (*SkInterruptHandler)(void *context);\n\n");
    out.push_str("typedef struct {\n");
    out.push_str("    int64_t period_ns;\n");
    out.push_str("    SkInterruptHandler handler;\n");
    out.push_str("    void *context;\n");
    out.push_str("    bool started;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    HANDLE thread;\n");
    out.push_str("    volatile LONG stopping;\n");
    out.push_str("#else\n");
    out.push_str("    pthread_t thread;\n");
    out.push_str("    pthread_mutex_t lock;\n");
    out.push_str("    bool stopping;\n");
    out.push_str("#endif\n");
    out.push_str("} SkInterrupt;\n\n");
    out.push_str("static void sk_interrupt_panic(const char *message) {\n");
    out.push_str("    fprintf(stderr, \"Runtime error: [SC-RT-320] %s\\n\", message);\n");
    out.push_str("    exit(1);\n");
    out.push_str("}\n\n");
    out.push_str("static bool sk_interrupt_is_stopping(SkInterrupt *interrupt) {\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    return InterlockedCompareExchange(&interrupt->stopping, 0, 0) != 0;\n");
    out.push_str("#else\n");
    out.push_str("    bool stopping;\n");
    out.push_str("    if (pthread_mutex_lock(&interrupt->lock) != 0) sk_interrupt_panic(\"interrupt lock failed\");\n");
    out.push_str("    stopping = interrupt->stopping;\n");
    out.push_str("    if (pthread_mutex_unlock(&interrupt->lock) != 0) sk_interrupt_panic(\"interrupt unlock failed\");\n");
    out.push_str("    return stopping;\n");
    out.push_str("#endif\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_interrupt_sleep(int64_t duration_ns) {\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    DWORD millis = (DWORD)((duration_ns + 999999LL) / 1000000LL);\n");
    out.push_str("    Sleep(millis > 0 ? millis : 1);\n");
    out.push_str("#else\n");
    out.push_str("    struct timespec request;\n");
    out.push_str("    request.tv_sec = (time_t)(duration_ns / 1000000000LL);\n");
    out.push_str("    request.tv_nsec = (long)(duration_ns % 1000000000LL);\n");
    out.push_str("    while (nanosleep(&request, &request) != 0 && errno == EINTR) {}\n");
    out.push_str("#endif\n");
    out.push_str("}\n\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("static DWORD WINAPI sk_interrupt_thread_main(LPVOID opaque) {\n");
    out.push_str("#else\n");
    out.push_str("static void* sk_interrupt_thread_main(void *opaque) {\n");
    out.push_str("#endif\n");
    out.push_str("    SkInterrupt *interrupt = (SkInterrupt*)opaque;\n");
    out.push_str("    while (!sk_interrupt_is_stopping(interrupt)) {\n");
    out.push_str("        sk_interrupt_sleep(interrupt->period_ns);\n");
    out.push_str("        if (!sk_interrupt_is_stopping(interrupt)) interrupt->handler(interrupt->context);\n");
    out.push_str("    }\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    return 0;\n");
    out.push_str("#else\n");
    out.push_str("    return NULL;\n");
    out.push_str("#endif\n");
    out.push_str("}\n\n");
    out.push_str("static SkInterrupt* sk_interrupt_periodic(int64_t period_ns) {\n");
    out.push_str("    if (period_ns <= 0) sk_interrupt_panic(\"periodic interrupt duration must be positive\");\n");
    out.push_str("    SkInterrupt *interrupt = (SkInterrupt*)calloc(1, sizeof(SkInterrupt));\n");
    out.push_str("    if (!interrupt) sk_interrupt_panic(\"interrupt allocation failed\");\n");
    out.push_str("    interrupt->period_ns = period_ns;\n");
    out.push_str("#if !defined(_WIN32)\n");
    out.push_str("    if (pthread_mutex_init(&interrupt->lock, NULL) != 0) { free(interrupt); sk_interrupt_panic(\"interrupt mutex initialization failed\"); }\n");
    out.push_str("#endif\n");
    out.push_str("    return interrupt;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_interrupt_bind(SkInterrupt *interrupt, SkInterruptHandler handler, void *context) {\n");
    out.push_str("    if (!interrupt || !handler || interrupt->started) sk_interrupt_panic(\"invalid or duplicate interrupt binding\");\n");
    out.push_str("    interrupt->handler = handler;\n");
    out.push_str("    interrupt->context = context;\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("    interrupt->thread = CreateThread(NULL, 0, sk_interrupt_thread_main, interrupt, 0, NULL);\n");
    out.push_str(
        "    if (!interrupt->thread) sk_interrupt_panic(\"interrupt thread creation failed\");\n",
    );
    out.push_str("#else\n");
    out.push_str("    if (pthread_create(&interrupt->thread, NULL, sk_interrupt_thread_main, interrupt) != 0) sk_interrupt_panic(\"interrupt thread creation failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    interrupt->started = true;\n");
    out.push_str("}\n\n");
    out.push_str("static void sk_interrupt_destroy(SkInterrupt *interrupt) {\n");
    out.push_str("    if (!interrupt) return;\n");
    out.push_str("    if (interrupt->started) {\n");
    out.push_str("#if defined(_WIN32)\n");
    out.push_str("        InterlockedExchange(&interrupt->stopping, 1);\n");
    out.push_str("        WaitForSingleObject(interrupt->thread, INFINITE);\n");
    out.push_str("        CloseHandle(interrupt->thread);\n");
    out.push_str("#else\n");
    out.push_str("        if (pthread_mutex_lock(&interrupt->lock) != 0) sk_interrupt_panic(\"interrupt stop lock failed\");\n");
    out.push_str("        interrupt->stopping = true;\n");
    out.push_str("        if (pthread_mutex_unlock(&interrupt->lock) != 0) sk_interrupt_panic(\"interrupt stop unlock failed\");\n");
    out.push_str("        if (pthread_join(interrupt->thread, NULL) != 0) sk_interrupt_panic(\"interrupt join failed\");\n");
    out.push_str("#endif\n");
    out.push_str("    }\n");
    out.push_str("#if !defined(_WIN32)\n");
    out.push_str("    pthread_mutex_destroy(&interrupt->lock);\n");
    out.push_str("#endif\n");
    out.push_str("    free(interrupt->context);\n");
    out.push_str("    free(interrupt);\n");
    out.push_str("}\n\n");
    out.push_str("static SkInterrupt* sk_interrupt_move(SkInterrupt **source) {\n");
    out.push_str("    if (!source) return NULL;\n");
    out.push_str("    SkInterrupt *result = *source;\n");
    out.push_str("    *source = NULL;\n");
    out.push_str("    return result;\n");
    out.push_str("}\n\n");
}

fn emit_visual_runtime(out: &mut String, needs_window_runtime: bool) {
    out.push_str(
        r##"typedef struct { uint8_t r; uint8_t g; uint8_t b; uint8_t a; } SkColor;
typedef struct { double x; double y; double width; double height; } SkRect;
typedef struct {
    int64_t width;
    int64_t height;
    uint32_t *pixels;
} SkCanvas;

static void sk_visual_panic(const char *message) {
    fprintf(stderr, "Runtime error: [SC-RT-330] %s\n", message);
    exit(1);
}

static SkColor sk_color(int64_t r, int64_t g, int64_t b, int64_t a) {
    if (r < 0 || r > 255 || g < 0 || g > 255 || b < 0 || b > 255 || a < 0 || a > 255) {
        sk_visual_panic("color components must be in 0..255");
    }
    return (SkColor){(uint8_t)r, (uint8_t)g, (uint8_t)b, (uint8_t)a};
}

static int sk_hex_digit(char value) {
    if (value >= '0' && value <= '9') return value - '0';
    if (value >= 'a' && value <= 'f') return value - 'a' + 10;
    if (value >= 'A' && value <= 'F') return value - 'A' + 10;
    return -1;
}

static SkColor sk_color_hex(const char *text, int64_t alpha) {
    if (!text) sk_visual_panic("color_hex expects rrggbb or #rrggbb text");
    if (text[0] == '#') text += 1;
    if (strlen(text) != 6) sk_visual_panic("color_hex expects exactly six hexadecimal digits");
    int digits[6];
    for (int index = 0; index < 6; ++index) {
        digits[index] = sk_hex_digit(text[index]);
        if (digits[index] < 0) sk_visual_panic("color_hex contains a non-hexadecimal digit");
    }
    return sk_color(
        digits[0] * 16 + digits[1],
        digits[2] * 16 + digits[3],
        digits[4] * 16 + digits[5],
        alpha
    );
}

#define Color_terminal_black ((SkColor){29, 31, 33, 255})
#define Color_terminal_red ((SkColor){204, 102, 102, 255})
#define Color_terminal_green ((SkColor){181, 189, 104, 255})
#define Color_terminal_yellow ((SkColor){240, 198, 116, 255})
#define Color_terminal_blue ((SkColor){129, 162, 190, 255})
#define Color_terminal_magenta ((SkColor){178, 148, 187, 255})
#define Color_terminal_cyan ((SkColor){138, 190, 183, 255})
#define Color_terminal_white ((SkColor){197, 200, 198, 255})
#define Color_terminal_bright_black ((SkColor){102, 102, 102, 255})
#define Color_terminal_bright_red ((SkColor){213, 78, 83, 255})
#define Color_terminal_bright_green ((SkColor){185, 202, 74, 255})
#define Color_terminal_bright_yellow ((SkColor){231, 197, 71, 255})
#define Color_terminal_bright_blue ((SkColor){122, 166, 218, 255})
#define Color_terminal_bright_magenta ((SkColor){195, 151, 216, 255})
#define Color_terminal_bright_cyan ((SkColor){112, 192, 177, 255})
#define Color_terminal_bright_white ((SkColor){234, 234, 234, 255})
#define Color_black Color_terminal_black
#define Color_red Color_terminal_red
#define Color_green Color_terminal_green
#define Color_yellow Color_terminal_yellow
#define Color_blue Color_terminal_blue
#define Color_magenta Color_terminal_magenta
#define Color_cyan Color_terminal_cyan
#define Color_white Color_terminal_bright_white
#define Color_transparent ((SkColor){0, 0, 0, 0})

static SkRect sk_rect(double x, double y, double width, double height) {
    return (SkRect){x, y, width, height};
}

static int64_t sk_visual_round(double value) {
    return (int64_t)(value >= 0.0 ? value + 0.5 : value - 0.5);
}

static uint32_t sk_color_pack(SkColor color) {
    return ((uint32_t)color.a << 24) | ((uint32_t)color.r << 16)
        | ((uint32_t)color.g << 8) | (uint32_t)color.b;
}

static SkColor sk_color_unpack(uint32_t value) {
    return (SkColor){
        (uint8_t)((value >> 16) & 255),
        (uint8_t)((value >> 8) & 255),
        (uint8_t)(value & 255),
        (uint8_t)((value >> 24) & 255)
    };
}

static SkCanvas sk_canvas_create(int64_t width, int64_t height) {
    if (width <= 0 || height <= 0 || (uint64_t)width > SIZE_MAX / sizeof(uint32_t)
        || (uint64_t)height > SIZE_MAX / ((size_t)width * sizeof(uint32_t))) {
        sk_visual_panic("canvas dimensions must be positive and fit addressable memory");
    }
    SkCanvas canvas = {width, height, NULL};
    canvas.pixels = (uint32_t*)calloc((size_t)width * (size_t)height, sizeof(uint32_t));
    if (!canvas.pixels) sk_visual_panic("canvas framebuffer allocation failed");
    return canvas;
}

static void sk_canvas_destroy(SkCanvas *canvas) {
    if (!canvas) return;
    free(canvas->pixels);
    canvas->pixels = NULL;
    canvas->width = 0;
    canvas->height = 0;
}

static SkCanvas sk_canvas_move(SkCanvas *source) {
    if (!source) {
        SkCanvas empty = {0};
        return empty;
    }
    SkCanvas result = *source;
    source->width = 0;
    source->height = 0;
    source->pixels = NULL;
    return result;
}

static void sk_canvas_blend_pixel(SkCanvas *canvas, int64_t x, int64_t y, SkColor source) {
    if (!canvas || !canvas->pixels || x < 0 || y < 0 || x >= canvas->width || y >= canvas->height) return;
    size_t index = (size_t)y * (size_t)canvas->width + (size_t)x;
    if (source.a == 255) {
        canvas->pixels[index] = sk_color_pack(source);
        return;
    }
    if (source.a == 0) return;
    SkColor target = sk_color_unpack(canvas->pixels[index]);
    uint32_t alpha = source.a;
    uint32_t inverse = 255 - alpha;
    SkColor blended = {
        (uint8_t)((source.r * alpha + target.r * inverse + 127) / 255),
        (uint8_t)((source.g * alpha + target.g * inverse + 127) / 255),
        (uint8_t)((source.b * alpha + target.b * inverse + 127) / 255),
        (uint8_t)(alpha + (target.a * inverse + 127) / 255)
    };
    canvas->pixels[index] = sk_color_pack(blended);
}

static int64_t sk_canvas_clear(SkCanvas *canvas, SkColor color) {
    if (!canvas || !canvas->pixels) sk_visual_panic("clear requires a live Canvas");
    uint32_t packed = sk_color_pack(color);
    size_t count = (size_t)canvas->width * (size_t)canvas->height;
    for (size_t index = 0; index < count; ++index) canvas->pixels[index] = packed;
    return 0;
}

static int64_t sk_canvas_pixel(SkCanvas *canvas, Vec2 position, SkColor color) {
    sk_canvas_blend_pixel(canvas, sk_visual_round(position.x), sk_visual_round(position.y), color);
    return 0;
}

static int64_t sk_canvas_line(SkCanvas *canvas, Vec2 from, Vec2 to, SkColor color) {
    int64_t x0 = sk_visual_round(from.x), y0 = sk_visual_round(from.y);
    int64_t x1 = sk_visual_round(to.x), y1 = sk_visual_round(to.y);
    int64_t dx = x1 >= x0 ? x1 - x0 : x0 - x1;
    int64_t sx = x0 < x1 ? 1 : -1;
    int64_t dy_abs = y1 >= y0 ? y1 - y0 : y0 - y1;
    int64_t dy = -dy_abs;
    int64_t sy = y0 < y1 ? 1 : -1;
    int64_t error = dx + dy;
    for (;;) {
        sk_canvas_blend_pixel(canvas, x0, y0, color);
        if (x0 == x1 && y0 == y1) break;
        int64_t doubled = 2 * error;
        if (doubled >= dy) { error += dy; x0 += sx; }
        if (doubled <= dx) { error += dx; y0 += sy; }
    }
    return 0;
}

static int64_t sk_canvas_fill_rect(SkCanvas *canvas, SkRect area, SkColor color) {
    if (area.width <= 0.0 || area.height <= 0.0) return 0;
    int64_t left = sk_visual_round(area.x);
    int64_t top = sk_visual_round(area.y);
    int64_t right = sk_visual_round(area.x + area.width) - 1;
    int64_t bottom = sk_visual_round(area.y + area.height) - 1;
    for (int64_t y = top; y <= bottom; ++y)
        for (int64_t x = left; x <= right; ++x)
            sk_canvas_blend_pixel(canvas, x, y, color);
    return 0;
}

static int64_t sk_canvas_rect(SkCanvas *canvas, SkRect area, SkColor color) {
    if (area.width <= 0.0 || area.height <= 0.0) return 0;
    int64_t left = sk_visual_round(area.x);
    int64_t top = sk_visual_round(area.y);
    int64_t right = sk_visual_round(area.x + area.width) - 1;
    int64_t bottom = sk_visual_round(area.y + area.height) - 1;
    for (int64_t x = left; x <= right; ++x) {
        sk_canvas_blend_pixel(canvas, x, top, color);
        sk_canvas_blend_pixel(canvas, x, bottom, color);
    }
    for (int64_t y = top; y <= bottom; ++y) {
        sk_canvas_blend_pixel(canvas, left, y, color);
        sk_canvas_blend_pixel(canvas, right, y, color);
    }
    return 0;
}

static void sk_canvas_circle_octants(SkCanvas *canvas, int64_t cx, int64_t cy, int64_t x, int64_t y, SkColor color) {
    sk_canvas_blend_pixel(canvas, cx + x, cy + y, color);
    sk_canvas_blend_pixel(canvas, cx + y, cy + x, color);
    sk_canvas_blend_pixel(canvas, cx - y, cy + x, color);
    sk_canvas_blend_pixel(canvas, cx - x, cy + y, color);
    sk_canvas_blend_pixel(canvas, cx - x, cy - y, color);
    sk_canvas_blend_pixel(canvas, cx - y, cy - x, color);
    sk_canvas_blend_pixel(canvas, cx + y, cy - x, color);
    sk_canvas_blend_pixel(canvas, cx + x, cy - y, color);
}

static int64_t sk_canvas_circle(SkCanvas *canvas, Vec2 center, double radius, SkColor color) {
    if (radius < 0.0) sk_visual_panic("circle radius cannot be negative");
    int64_t cx = sk_visual_round(center.x), cy = sk_visual_round(center.y);
    int64_t x = sk_visual_round(radius), y = 0, error = 1 - x;
    while (x >= y) {
        sk_canvas_circle_octants(canvas, cx, cy, x, y, color);
        ++y;
        if (error < 0) error += 2 * y + 1;
        else { --x; error += 2 * (y - x) + 1; }
    }
    return 0;
}

static int64_t sk_canvas_fill_circle(SkCanvas *canvas, Vec2 center, double radius, SkColor color) {
    if (radius < 0.0) sk_visual_panic("fill_circle radius cannot be negative");
    int64_t cx = sk_visual_round(center.x), cy = sk_visual_round(center.y);
    int64_t r = sk_visual_round(radius);
    int64_t radius_squared = r * r;
    for (int64_t y = -r; y <= r; ++y)
        for (int64_t x = -r; x <= r; ++x)
            if (x * x + y * y <= radius_squared)
                sk_canvas_blend_pixel(canvas, cx + x, cy + y, color);
    return 0;
}

static int64_t sk_canvas_checksum(const SkCanvas *canvas) {
    if (!canvas || !canvas->pixels) sk_visual_panic("checksum requires a live Canvas");
    uint64_t hash = 1469598103934665603ULL;
    size_t bytes = (size_t)canvas->width * (size_t)canvas->height * sizeof(uint32_t);
    const unsigned char *data = (const unsigned char*)canvas->pixels;
    for (size_t index = 0; index < bytes; ++index) {
        hash ^= data[index];
        hash *= 1099511628211ULL;
    }
    return (int64_t)(hash & 0x7fffffffffffffffULL);
}

"##,
    );
    if !needs_window_runtime {
        return;
    }
    out.push_str(
        r##"typedef struct {
    bool open;
    int64_t width;
    int64_t height;
#if defined(_WIN32)
    HWND hwnd;
    BITMAPINFO bitmap;
#endif
} SkWindow;

#if defined(_WIN32)
static LRESULT CALLBACK sk_window_proc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
    SkWindow *window = (SkWindow*)GetWindowLongPtrA(hwnd, GWLP_USERDATA);
    if (message == WM_NCCREATE) {
        CREATESTRUCTA *create = (CREATESTRUCTA*)lparam;
        window = (SkWindow*)create->lpCreateParams;
        SetWindowLongPtrA(hwnd, GWLP_USERDATA, (LONG_PTR)window);
        window->hwnd = hwnd;
    }
    if (message == WM_CLOSE) {
        if (window) window->open = false;
        DestroyWindow(hwnd);
        return 0;
    }
    if (message == WM_DESTROY) {
        if (window) { window->open = false; window->hwnd = NULL; }
        return 0;
    }
    return DefWindowProcA(hwnd, message, wparam, lparam);
}
#endif

static SkWindow sk_window_open(const char *title, int64_t width, int64_t height) {
    if (width <= 0 || height <= 0) sk_visual_panic("window dimensions must be positive");
    SkWindow window = {true, width, height
#if defined(_WIN32)
        , NULL, {0}
#endif
    };
#if defined(_WIN32)
    static bool class_registered = false;
    const char *class_name = "SkadiCanvasWindow";
    HINSTANCE instance = GetModuleHandleA(NULL);
    if (!class_registered) {
        WNDCLASSA klass = {0};
        klass.lpfnWndProc = sk_window_proc;
        klass.hInstance = instance;
        klass.lpszClassName = class_name;
        klass.hCursor = LoadCursor(NULL, IDC_ARROW);
        if (!RegisterClassA(&klass) && GetLastError() != ERROR_CLASS_ALREADY_EXISTS)
            sk_visual_panic("Win32 window class registration failed");
        class_registered = true;
    }
    RECT bounds = {0, 0, (LONG)width, (LONG)height};
    AdjustWindowRect(&bounds, WS_OVERLAPPEDWINDOW, FALSE);
    HWND hwnd = CreateWindowExA(
        0, class_name, title ? title : "Skadi Canvas", WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT, CW_USEDEFAULT, bounds.right - bounds.left, bounds.bottom - bounds.top,
        NULL, NULL, instance, &window
    );
    if (!hwnd) sk_visual_panic("Win32 window creation failed");
    window.hwnd = hwnd;
    window.bitmap.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    window.bitmap.bmiHeader.biWidth = (LONG)width;
    window.bitmap.bmiHeader.biHeight = -(LONG)height;
    window.bitmap.bmiHeader.biPlanes = 1;
    window.bitmap.bmiHeader.biBitCount = 32;
    window.bitmap.bmiHeader.biCompression = BI_RGB;
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
#else
    (void)title;
    sk_visual_panic("windows.open is available only on the Win32 backend");
#endif
    return window;
}

static int64_t sk_window_present(SkWindow *window, const SkCanvas *canvas) {
    if (!window || !canvas || !canvas->pixels) sk_visual_panic("present requires live Window and Canvas");
    if (window->width != canvas->width || window->height != canvas->height)
        sk_visual_panic("Window and Canvas dimensions must match");
#if defined(_WIN32)
    MSG message;
    while (PeekMessageA(&message, NULL, 0, 0, PM_REMOVE)) {
        TranslateMessage(&message);
        DispatchMessageA(&message);
    }
    if (!window->open || !window->hwnd) return 1;
    HDC dc = GetDC(window->hwnd);
    if (!dc) sk_visual_panic("Win32 device context acquisition failed");
    StretchDIBits(
        dc, 0, 0, (int)window->width, (int)window->height,
        0, 0, (int)canvas->width, (int)canvas->height,
        canvas->pixels, &window->bitmap, DIB_RGB_COLORS, SRCCOPY
    );
    ReleaseDC(window->hwnd, dc);
#endif
    return 0;
}

static bool sk_window_is_open(SkWindow *window) {
#if defined(_WIN32)
    MSG message;
    while (PeekMessageA(&message, NULL, 0, 0, PM_REMOVE)) {
        TranslateMessage(&message);
        DispatchMessageA(&message);
    }
#endif
    return window && window->open;
}

static int64_t sk_window_close(SkWindow *window) {
    if (!window || !window->open) return 1;
    window->open = false;
#if defined(_WIN32)
    if (window->hwnd) DestroyWindow(window->hwnd);
    window->hwnd = NULL;
#endif
    return 0;
}

static void sk_window_destroy(SkWindow *window) {
    if (!window) return;
    sk_window_close(window);
}

static SkWindow sk_window_move(SkWindow *source) {
    if (!source) {
        SkWindow empty = {0};
        return empty;
    }
    SkWindow result = *source;
    memset(source, 0, sizeof(*source));
    return result;
}

"##,
    );
}

fn program_uses_interrupt_runtime(program: &Program) -> bool {
    program.statements.iter().any(|statement| {
        matches!(
            statement,
            Statement::VarDecl {
                declared_type: Some(declared_type),
                ..
            } if normalize_type_token(declared_type) == "Interrupt"
        ) || matches!(
            statement,
            Statement::OnBlock { trigger, .. } if trigger == "interrupt"
        )
    })
}

fn program_uses_visual_runtime(program: &Program) -> bool {
    fn visual_type(raw: &str) -> bool {
        matches!(
            normalize_type_token(raw).as_str(),
            "Color" | "Rect" | "Canvas" | "Window"
        )
    }

    fn statement_uses_visual(statement: &Statement) -> bool {
        match statement {
            Statement::VarDecl {
                declared_type,
                value,
                ..
            } => {
                declared_type.as_deref().map(visual_type).unwrap_or(false)
                    || matches!(
                        value.as_ref(),
                        Expression::Call { name, .. }
                            if matches!(
                                name.as_str(),
                                "color" | "color_hex" | "rect" | "canvas" | "windows.open"
                            )
                    )
            }
            Statement::FunctionDef {
                params,
                returns,
                body,
                ..
            } => {
                params
                    .iter()
                    .filter_map(|param| param.param_type.as_deref())
                    .any(visual_type)
                    || returns.as_deref().map(visual_type).unwrap_or(false)
                    || body.statements.iter().any(statement_uses_visual)
            }
            Statement::StructDecl {
                fields, methods, ..
            } => {
                fields.iter().any(|field| visual_type(&field.field_type))
                    || methods
                        .iter()
                        .any(|method| method.body.statements.iter().any(statement_uses_visual))
            }
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                then_block.statements.iter().any(statement_uses_visual)
                    || else_block
                        .as_deref()
                        .map(|block| block.statements.iter().any(statement_uses_visual))
                        .unwrap_or(false)
            }
            Statement::WhenBlock {
                cases, else_block, ..
            } => {
                cases
                    .iter()
                    .any(|(_, block)| block.statements.iter().any(statement_uses_visual))
                    || else_block
                        .as_deref()
                        .map(|block| block.statements.iter().any(statement_uses_visual))
                        .unwrap_or(false)
            }
            Statement::ForLoop { body, .. }
            | Statement::WhileLoop { body, .. }
            | Statement::LoopStatement { body, .. }
            | Statement::OnBlock { body, .. } => body.statements.iter().any(statement_uses_visual),
            Statement::PlaceIn { body, on_error, .. } => {
                body.statements.iter().any(statement_uses_visual)
                    || on_error
                        .as_deref()
                        .map(|block| block.statements.iter().any(statement_uses_visual))
                        .unwrap_or(false)
            }
            Statement::DangerAssignOnError { on_error, .. }
            | Statement::DangerCallOnError { on_error, .. }
            | Statement::ListPopOnError { on_error, .. } => {
                on_error.statements.iter().any(statement_uses_visual)
            }
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => {
                statements.iter().any(statement_uses_visual)
            }
            Statement::ExpressionStatement { expr, .. } => matches!(
                expr.as_ref(),
                Expression::Call { name, .. }
                    if name.split_once('.').map(|(_, method)| {
                        matches!(
                            method,
                            "clear"
                                | "pixel"
                                | "line"
                                | "rect"
                                | "fill_rect"
                                | "circle"
                                | "fill_circle"
                                | "checksum"
                                | "present"
                                | "is_open"
                                | "close"
                        )
                    }).unwrap_or(false)
            ),
            _ => false,
        }
    }

    program.statements.iter().any(statement_uses_visual)
}

fn program_uses_window_runtime(program: &Program) -> bool {
    fn window_type(raw: &str) -> bool {
        normalize_type_token(raw) == "Window"
    }

    fn statement_uses_window(statement: &Statement) -> bool {
        match statement {
            Statement::VarDecl {
                declared_type,
                value,
                ..
            } => {
                declared_type.as_deref().map(window_type).unwrap_or(false)
                    || matches!(
                        value.as_ref(),
                        Expression::Call { name, .. } if name == "windows.open"
                    )
            }
            Statement::FunctionDef {
                params,
                returns,
                body,
                ..
            } => {
                params
                    .iter()
                    .filter_map(|param| param.param_type.as_deref())
                    .any(window_type)
                    || returns.as_deref().map(window_type).unwrap_or(false)
                    || body.statements.iter().any(statement_uses_window)
            }
            Statement::StructDecl {
                fields, methods, ..
            } => {
                fields.iter().any(|field| window_type(&field.field_type))
                    || methods
                        .iter()
                        .any(|method| method.body.statements.iter().any(statement_uses_window))
            }
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                then_block.statements.iter().any(statement_uses_window)
                    || else_block
                        .as_deref()
                        .map(|block| block.statements.iter().any(statement_uses_window))
                        .unwrap_or(false)
            }
            Statement::WhenBlock {
                cases, else_block, ..
            } => {
                cases
                    .iter()
                    .any(|(_, block)| block.statements.iter().any(statement_uses_window))
                    || else_block
                        .as_deref()
                        .map(|block| block.statements.iter().any(statement_uses_window))
                        .unwrap_or(false)
            }
            Statement::ForLoop { body, .. }
            | Statement::WhileLoop { body, .. }
            | Statement::LoopStatement { body, .. }
            | Statement::OnBlock { body, .. } => body.statements.iter().any(statement_uses_window),
            Statement::PlaceIn { body, on_error, .. } => {
                body.statements.iter().any(statement_uses_window)
                    || on_error
                        .as_deref()
                        .map(|block| block.statements.iter().any(statement_uses_window))
                        .unwrap_or(false)
            }
            Statement::DangerAssignOnError { on_error, .. }
            | Statement::DangerCallOnError { on_error, .. }
            | Statement::ListPopOnError { on_error, .. } => {
                on_error.statements.iter().any(statement_uses_window)
            }
            Statement::BlockStatement { statements, .. }
            | Statement::OnErrorBlock { statements, .. } => {
                statements.iter().any(statement_uses_window)
            }
            _ => false,
        }
    }

    program.statements.iter().any(statement_uses_window)
}

fn collect_interrupt_channels(block: &BlockStatement, channels: &mut Vec<String>) {
    for statement in &block.statements {
        match statement {
            Statement::ExpressionStatement { expr, .. } => {
                if let Expression::Call { name, .. } = expr.as_ref()
                    && let Some((channel, "try_send")) = name.split_once('.')
                    && !channels.iter().any(|existing| existing == channel)
                {
                    channels.push(channel.to_string());
                }
            }
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                collect_interrupt_channels(then_block, channels);
                if let Some(else_block) = else_block {
                    collect_interrupt_channels(else_block, channels);
                }
            }
            _ => {}
        }
    }
}

fn emit_interrupt_handlers(program: &Program, out: &mut String, state: &mut CodegenState) {
    let top_level_types: HashMap<String, String> = program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::VarDecl {
                name,
                declared_type: Some(declared_type),
                ..
            } => Some((name.clone(), declared_type.clone())),
            _ => None,
        })
        .collect();

    for (index, statement) in program
        .statements
        .iter()
        .filter(|statement| {
            matches!(
                statement,
                Statement::OnBlock { trigger, .. } if trigger == "interrupt"
            )
        })
        .enumerate()
    {
        let Statement::OnBlock { body, .. } = statement else {
            continue;
        };
        let mut channels = Vec::new();
        collect_interrupt_channels(body, &mut channels);
        out.push_str(&format!("typedef struct SkInterruptContext_{index} {{\n"));
        for channel in &channels {
            out.push_str("    SkChannel *");
            out.push_str(channel);
            out.push_str(";\n");
        }
        out.push_str(&format!("}} SkInterruptContext_{index};\n\n"));
        out.push_str(&format!(
            "static void sk_interrupt_handler_{index}(void *opaque) {{\n"
        ));
        out.push_str(&format!(
            "    SkInterruptContext_{index} *context = (SkInterruptContext_{index}*)opaque;\n"
        ));
        let mut declared = HashMap::new();
        for channel in &channels {
            out.push_str("    SkChannel *");
            out.push_str(channel);
            out.push_str(" = context->");
            out.push_str(channel);
            out.push_str(";\n");
            if let Some(declared_type) = top_level_types.get(channel) {
                declared.insert(channel.clone(), declared_type.clone());
            }
        }
        emit_block(body, out, 1, &mut declared, None, None, state);
        out.push_str("}\n\n");
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
            let c_type = map_skadi_type_to_c(p.param_type.as_deref());
            match p.borrow {
                BorrowMode::Value => out.push_str(&c_type),
                BorrowMode::DirectMutable => {
                    out.push_str(&c_type);
                    out.push_str(" *");
                }
                BorrowMode::View => {
                    out.push_str("const ");
                    out.push_str(&c_type);
                    out.push_str(" *");
                }
                BorrowMode::Move => out.push_str(&c_type),
            }
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
                (p.name.clone(), {
                    let base = p.param_type.clone().unwrap_or_else(|| "Int".to_string());
                    match p.borrow {
                        BorrowMode::Value => base,
                        BorrowMode::Move => format!("{base}@owned"),
                        BorrowMode::DirectMutable | BorrowMode::View => {
                            format!("{base}@direct")
                        }
                    }
                })
            })
            .collect();
        let fn_ctx = FunctionContext {
            is_danger: *is_danger,
            return_type: returns.clone(),
        };
        emit_block(body, out, 1, &mut declared, Some(&fn_ctx), None, state);
        emit_move_parameter_cleanup(out, 1, params);
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
    for resource_type in ["Window", "Canvas"] {
        for stmt in block.statements.iter().rev() {
            if let Statement::VarDecl {
                name,
                declared_type: Some(declared_type),
                ..
            } = stmt
                && normalize_type_token(declared_type) == resource_type
            {
                out.push_str(&"    ".repeat(indent));
                out.push_str(if resource_type == "Window" {
                    "sk_window_destroy(&"
                } else {
                    "sk_canvas_destroy(&"
                });
                out.push_str(name);
                out.push_str(");\n");
            }
        }
    }
    for stmt in block.statements.iter().rev() {
        if let Statement::VarDecl {
            name,
            declared_type: Some(declared_type),
            ..
        } = stmt
            && normalize_type_token(declared_type) == "Interrupt"
        {
            out.push_str(&"    ".repeat(indent));
            out.push_str("sk_interrupt_destroy(");
            out.push_str(name);
            out.push_str(");\n");
        }
    }
    for stmt in block.statements.iter().rev() {
        if let Statement::MemoryDecl { name, .. } = stmt {
            out.push_str(&"    ".repeat(indent));
            out.push_str("sk_mem_region_destroy(");
            out.push_str(name);
            out.push_str(");\n");
        }
    }
}

fn emit_move_parameter_cleanup(
    out: &mut String,
    indent: usize,
    params: &[crate::ast_nodes::FunctionParam],
) {
    let pad = "    ".repeat(indent);
    for param in params
        .iter()
        .rev()
        .filter(|param| param.borrow == BorrowMode::Move)
    {
        let param_type = param.param_type.as_deref().unwrap_or("Int");
        let normalized = normalize_type_token(param_type);
        out.push_str(&pad);
        match normalized.as_str() {
            "Canvas" => {
                out.push_str("sk_canvas_destroy(&");
                out.push_str(&param.name);
                out.push_str(");\n");
            }
            "Window" => {
                out.push_str("sk_window_destroy(&");
                out.push_str(&param.name);
                out.push_str(");\n");
            }
            "Interrupt" => {
                out.push_str("sk_interrupt_destroy(");
                out.push_str(&param.name);
                out.push_str(");\n");
            }
            _ if channel_elem_from_decl(&normalized).is_some() => {
                out.push_str("sk_channel_destroy(");
                out.push_str(&param.name);
                out.push_str(");\n");
            }
            _ => {}
        }
    }
}

fn emit_owned_channel_cleanup(out: &mut String, pad: &str, declared: &HashMap<String, String>) {
    let mut memory_names = declared
        .iter()
        .filter_map(|(name, declared_type)| {
            (declared_type == "Memory@owned").then_some(name.as_str())
        })
        .collect::<Vec<_>>();
    memory_names.sort_unstable();
    for memory_name in memory_names.into_iter().rev() {
        out.push_str(pad);
        out.push_str("sk_mem_region_destroy(");
        out.push_str(memory_name);
        out.push_str(");\n");
    }
    for resource_type in ["Window", "Canvas"] {
        let mut resources = declared
            .iter()
            .filter_map(|(name, declared_type)| {
                (declared_type.ends_with("@owned")
                    && normalize_type_token(declared_type) == resource_type)
                    .then_some(name.as_str())
            })
            .collect::<Vec<_>>();
        resources.sort_unstable();
        for resource in resources.into_iter().rev() {
            out.push_str(pad);
            out.push_str(if resource_type == "Window" {
                "sk_window_destroy(&"
            } else {
                "sk_canvas_destroy(&"
            });
            out.push_str(resource);
            out.push_str(");\n");
        }
    }
    let mut interrupt_names = declared
        .iter()
        .filter_map(|(name, declared_type)| {
            (declared_type.ends_with("@owned")
                && normalize_type_token(declared_type) == "Interrupt")
                .then_some(name.as_str())
        })
        .collect::<Vec<_>>();
    interrupt_names.sort_unstable();
    for interrupt_name in interrupt_names.into_iter().rev() {
        out.push_str(pad);
        out.push_str("sk_interrupt_destroy(");
        out.push_str(interrupt_name);
        out.push_str(");\n");
    }
    let mut channel_names: Vec<&str> = declared
        .iter()
        .filter_map(|(name, declared_type)| {
            (declared_type.ends_with("@owned") && channel_elem_from_decl(declared_type).is_some())
                .then_some(name.as_str())
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

fn infer_scalar_declaration_type(
    value: &Expression,
    declared: &HashMap<String, String>,
    state: &CodegenState,
) -> Option<String> {
    match value {
        Expression::LiteralDuration { .. } => return Some("Duration".to_string()),
        Expression::LiteralByteSize { .. } => return Some("ByteSize".to_string()),
        Expression::LiteralAngle { .. } => return Some("Angle".to_string()),
        Expression::VariableReference(name) => {
            if let Some(declared_type) = declared.get(name) {
                return Some(normalize_type_token(declared_type));
            }
        }
        Expression::Call { name, .. } => {
            if let Some(return_type) = state.function_returns.get(name) {
                return Some(normalize_type_token(return_type));
            }
        }
        _ => {}
    }

    match expr_kind(value, declared) {
        ExprKind::Int => Some("Int".to_string()),
        ExprKind::Float => Some("Float".to_string()),
        ExprKind::Bool => Some("Bool".to_string()),
        ExprKind::Char => Some("Char".to_string()),
        ExprKind::Text => Some("Text".to_string()),
        ExprKind::Unknown => None,
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
            size,
            kind,
            allow_grow,
            allow_drop,
            on_error,
            ..
        } => {
            let size_expr = emit_expr(size, declared);
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
            out.push_str("int64_t ");
            out.push_str(name);
            out.push_str("_capacity = ");
            out.push_str(&size_expr);
            out.push_str(";\n");
            if *kind == crate::ast_nodes::MemoryKind::Static {
                out.push_str(&pad);
                out.push_str("static unsigned char ");
                out.push_str(name);
                out.push_str("_buffer[");
                out.push_str(&size_expr);
                out.push_str("];\n");
            }
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(name);
            out.push_str("_capacity <= 0 || !");
            out.push_str(match kind {
                crate::ast_nodes::MemoryKind::Dynamic => "sk_mem_region_init(",
                crate::ast_nodes::MemoryKind::Child => "sk_mem_region_init_child(",
                crate::ast_nodes::MemoryKind::Static => "sk_mem_region_init_external(",
            });
            out.push_str(name);
            out.push_str(", ");
            if *kind == crate::ast_nodes::MemoryKind::Child {
                out.push_str("sk_mem_current(), ");
            } else if *kind == crate::ast_nodes::MemoryKind::Static {
                out.push_str(name);
                out.push_str("_buffer, ");
            }
            out.push_str("(size_t)");
            out.push_str(name);
            out.push_str("_capacity, ");
            match kind {
                crate::ast_nodes::MemoryKind::Dynamic => {
                    out.push_str(if *allow_grow { "true, " } else { "false, " });
                    out.push_str(if *allow_drop { "true" } else { "false" });
                }
                crate::ast_nodes::MemoryKind::Child => {
                    out.push_str(if *allow_drop { "true" } else { "false" });
                }
                crate::ast_nodes::MemoryKind::Static => {
                    out.push_str("false, false, ");
                    out.push_str(if *allow_drop { "true" } else { "false" });
                }
            }
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
            declared.insert(name.clone(), "Memory@owned".to_string());
        }
        Statement::Assignment { target, value, .. } => {
            let expr = if let Expression::StructConstruction { fields } = value.as_ref() {
                emit_struct_literal(fields, declared.get(target).map(String::as_str), declared)
            } else {
                emit_expr(value, declared)
            };
            out.push_str(&pad);
            if declared
                .get(target)
                .map(|ty| ty.ends_with("@direct"))
                .unwrap_or(false)
            {
                out.push_str("(*");
                out.push_str(target);
                out.push(')');
            } else {
                out.push_str(target);
            }
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
            if declared
                .get(target)
                .map(|ty| ty.ends_with("@direct"))
                .unwrap_or(false)
            {
                out.push_str("(*");
                out.push_str(target);
                out.push(')');
            } else {
                out.push_str(target);
            }
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
            } else if declared
                .get(object)
                .map(|ty| ty.ends_with("@direct"))
                .unwrap_or(false)
            {
                out.push_str(object);
                out.push_str("->");
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
        Statement::TagDecl { name, .. } => {
            out.push_str(&pad);
            out.push_str("/* tag ");
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
            if trigger != "interrupt" {
                out.push_str(&pad);
                out.push_str("/* unsupported on ");
                out.push_str(trigger);
                out.push_str(" */\n");
                return;
            }
            let Statement::OnBlock {
                target: Some(target),
                body,
                ..
            } = stmt
            else {
                return;
            };
            let index = state.interrupt_handler_index;
            state.interrupt_handler_index += 1;
            let mut channels = Vec::new();
            collect_interrupt_channels(body, &mut channels);
            out.push_str(&pad);
            out.push_str(&format!(
                "SkInterruptContext_{index} *sk_interrupt_context_{index} = (SkInterruptContext_{index}*)calloc(1, sizeof(SkInterruptContext_{index}));\n"
            ));
            out.push_str(&pad);
            out.push_str(&format!(
                "if (!sk_interrupt_context_{index}) sk_interrupt_panic(\"interrupt context allocation failed\");\n"
            ));
            for channel in channels {
                out.push_str(&pad);
                out.push_str(&format!(
                    "sk_interrupt_context_{index}->{channel} = {channel};\n"
                ));
            }
            out.push_str(&pad);
            out.push_str(&format!(
                "sk_interrupt_bind({target}, sk_interrupt_handler_{index}, sk_interrupt_context_{index});\n"
            ));
        }
        Statement::DangerAssignOnError {
            target,
            call_name,
            args,
            on_error,
            ..
        } => {
            if let Some((channel, "receive_for")) = call_name.split_once('.')
                && let Some(element) = declared
                    .get(channel)
                    .and_then(|declared_type| channel_elem_from_decl(declared_type))
                && args.len() == 1
            {
                let id = state.next_id();
                out.push_str(&pad);
                out.push_str(&format!("SkChannelStatus sk_channel_status_{id} = (SkChannelStatus)sk_channel_receive_for_{}({}, &{}, {});\n", channel_type_suffix(element), channel, target, emit_expr(&args[0], declared)));
                out.push_str(&pad);
                out.push_str(&format!(
                    "if (sk_channel_status_{id} != SK_CHANNEL_OK) {{\n"
                ));
                out.push_str(&pad);
                out.push_str(&format!(
                    "    SkChannelStatus sk_previous_status_{id} = sk_channel_last_status;\n"
                ));
                out.push_str(&pad);
                out.push_str(&format!(
                    "    sk_channel_last_status = sk_channel_status_{id};\n"
                ));
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
                out.push_str(&format!(
                    "    sk_channel_last_status = sk_previous_status_{id};\n"
                ));
                out.push_str(&pad);
                out.push_str("}\n");
                return;
            }
            if let Some((channel, "receive")) = call_name.split_once('.')
                && let Some(element) = declared
                    .get(channel)
                    .and_then(|declared_type| channel_elem_from_decl(declared_type))
            {
                out.push_str(&pad);
                out.push_str("if (!sk_channel_try_receive_");
                out.push_str(&channel_type_suffix(element));
                out.push('(');
                out.push_str(channel);
                out.push_str(", &");
                out.push_str(target);
                out.push_str(")) {\n");
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
                return;
            }
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
            if let Some((channel, "send_for")) = call_name.split_once('.')
                && let Some(element) = declared
                    .get(channel)
                    .and_then(|declared_type| channel_elem_from_decl(declared_type))
                && args.len() == 2
            {
                let id = state.next_id();
                out.push_str(&pad);
                out.push_str(&format!("SkChannelStatus sk_channel_status_{id} = (SkChannelStatus)sk_channel_send_for_{}({}, {}, {});\n", channel_type_suffix(element), channel, emit_expr(&args[0], declared), emit_expr(&args[1], declared)));
                out.push_str(&pad);
                out.push_str(&format!(
                    "if (sk_channel_status_{id} != SK_CHANNEL_OK) {{\n"
                ));
                out.push_str(&pad);
                out.push_str(&format!(
                    "    SkChannelStatus sk_previous_status_{id} = sk_channel_last_status;\n"
                ));
                out.push_str(&pad);
                out.push_str(&format!(
                    "    sk_channel_last_status = sk_channel_status_{id};\n"
                ));
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
                out.push_str(&format!(
                    "    sk_channel_last_status = sk_previous_status_{id};\n"
                ));
                out.push_str(&pad);
                out.push_str("}\n");
                return;
            }
            if let Some((window, method @ ("present" | "close"))) = call_name.split_once('.')
                && let Some(window_type) = declared.get(window)
                && normalize_type_token(window_type.strip_suffix("@direct").unwrap_or(window_type))
                    == "Window"
            {
                let receiver = if window_type.ends_with("@direct") {
                    window.to_string()
                } else {
                    format!("&{window}")
                };
                out.push_str(&pad);
                out.push_str("if (");
                out.push_str(if method == "present" {
                    "sk_window_present"
                } else {
                    "sk_window_close"
                });
                out.push('(');
                out.push_str(&receiver);
                for arg in args {
                    out.push_str(", ");
                    out.push_str(&emit_expr(arg, declared));
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
                return;
            }
            if let Some((channel, "close")) = call_name.split_once('.')
                && declared
                    .get(channel)
                    .and_then(|declared_type| channel_elem_from_decl(declared_type))
                    .is_some()
                && args.is_empty()
            {
                out.push_str(&pad);
                out.push_str("if (!sk_channel_close(");
                out.push_str(channel);
                out.push_str(")) {\n");
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
                return;
            }
            if let Some((channel, "send")) = call_name.split_once('.')
                && let Some(element) = declared
                    .get(channel)
                    .and_then(|declared_type| channel_elem_from_decl(declared_type))
                && args.len() == 1
            {
                out.push_str(&pad);
                out.push_str("if (sk_channel_send_");
                out.push_str(&channel_type_suffix(element));
                out.push('(');
                out.push_str(channel);
                out.push_str(", ");
                out.push_str(&emit_expr(&args[0], declared));
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
                return;
            }
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
            is_constant,
            declared_type,
            ..
        } => {
            let effective_type = declared_type
                .clone()
                .or_else(|| infer_scalar_declaration_type(value, declared, state));
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
                if *is_constant {
                    out.push_str("const ");
                }
                out.push_str(&map_skadi_type_to_c(Some(dt)));
                out.push(' ');
                out.push_str(name);
                out.push_str(" = ");
                out.push_str(&emit_struct_literal(fields, Some(dt), declared));
                out.push_str(";\n");
                declared.insert(name.clone(), dt.to_string());
                return;
            }
            if *is_constant {
                out.push_str("const ");
            }
            out.push_str(&map_skadi_type_to_c(effective_type.as_deref()));
            out.push(' ');
            out.push_str(name);
            out.push_str(" = ");
            out.push_str(&emit_expr(value, declared));
            out.push_str(";\n");
            let mut tracked_type = effective_type.unwrap_or_else(|| "Int".to_string());
            if matches!(
                normalize_type_token(&tracked_type).as_str(),
                "Canvas" | "Window" | "Interrupt"
            ) || channel_elem_from_decl(&tracked_type).is_some()
            {
                tracked_type.push_str("@owned");
            }
            declared.insert(name.clone(), tracked_type);
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
        "Int" | "i64" | "Time" | "Duration" | "ByteSize" => "int64_t".to_string(),
        "u8" => "uint8_t".to_string(),
        "u16" => "uint16_t".to_string(),
        "u32" => "uint32_t".to_string(),
        "u64" => "uint64_t".to_string(),
        "f32" => "float".to_string(),
        "Float" | "f64" => "double".to_string(),
        "Angle" => "double".to_string(),
        "Vec2" | "Vec3" | "Vec4" => normalized.to_string(),
        "Color" => "SkColor".to_string(),
        "Rect" => "SkRect".to_string(),
        "Canvas" => "SkCanvas".to_string(),
        "Window" => "SkWindow".to_string(),
        "bool" | "Bool" => "bool".to_string(),
        "char" | "Char" => "char".to_string(),
        "Memory" => "SkMemoryRegion*".to_string(),
        "Interrupt" => "SkInterrupt*".to_string(),
        "Text" | "Path" => "const char*".to_string(),
        other => other.to_string(),
    }
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
    let raw = raw
        .strip_suffix("@owned")
        .or_else(|| raw.strip_suffix("@direct"))
        .unwrap_or(raw);
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
        Expression::LiteralChar(_) => ExprKind::Char,
        Expression::LiteralString(_) => ExprKind::Text,
        Expression::LiteralDuration { .. } => ExprKind::Int,
        Expression::LiteralByteSize { .. } => ExprKind::Int,
        Expression::LiteralAngle { .. } => ExprKind::Float,
        Expression::VariableReference(name) => match declared.get(name).map(String::as_str) {
            Some("Float" | "f32" | "f64" | "Angle") => ExprKind::Float,
            Some("bool" | "Bool") => ExprKind::Bool,
            Some("char" | "Char") => ExprKind::Char,
            Some("Text" | "Path") => ExprKind::Text,
            Some(_) => ExprKind::Int,
            None if matches!(name.as_str(), "PI" | "TAU" | "E" | "EPSILON") => ExprKind::Float,
            None => ExprKind::Unknown,
        },
        Expression::Call { name, .. } => match name.as_str() {
            "contains" | "fs.is_dir" => ExprKind::Bool,
            "find" | "len" | "write" | "output" | "now" | "elapsed" | "sleep" | "delay" => {
                ExprKind::Int
            }
            "input" | "read" | "slice" | "concat" | "fs.join" => ExprKind::Text,
            "abs" | "min" | "max" | "clamp" | "floor" | "ceil" | "round" | "sin" | "cos"
            | "atan2" | "sqrt" | "root" | "deg_to_rad" | "rad_to_deg" | "dot" | "length"
            | "length_sq" | "distance" | "distance_sq" => ExprKind::Float,
            _ => ExprKind::Unknown,
        },
        Expression::MemberAccess { base, .. }
            if declared
                .get(base)
                .map(|ty| matches!(normalize_type_token(ty).as_str(), "Vec2" | "Vec3" | "Vec4"))
                .unwrap_or(false) =>
        {
            ExprKind::Float
        }
        Expression::Index { base, .. } => {
            if let Expression::VariableReference(name) = base.as_ref()
                && let Some(element) = declared.get(name).and_then(|ty| list_elem_from_decl(ty))
            {
                return match normalize_type_token(element).as_str() {
                    "Float" | "f32" | "f64" | "Angle" => ExprKind::Float,
                    "bool" | "Bool" => ExprKind::Bool,
                    "char" | "Char" => ExprKind::Char,
                    "Text" | "Path" => ExprKind::Text,
                    _ => ExprKind::Int,
                };
            }
            if is_text_expr(base, declared) {
                ExprKind::Char
            } else {
                ExprKind::Unknown
            }
        }
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

fn vector_expr_type(expr: &Expression, declared: &HashMap<String, String>) -> Option<&'static str> {
    let from_type_name = |ty: &str| match normalize_type_token(ty).as_str() {
        "Vec2" => Some("vec2"),
        "Vec3" => Some("vec3"),
        "Vec4" => Some("vec4"),
        _ => None,
    };
    match expr {
        Expression::VariableReference(name) => declared.get(name).and_then(|ty| from_type_name(ty)),
        Expression::Index { base, .. } => {
            let Expression::VariableReference(name) = base.as_ref() else {
                return None;
            };
            declared
                .get(name)
                .and_then(|ty| list_elem_from_decl(ty))
                .and_then(from_type_name)
        }
        Expression::Call { name, args } if name == "normalize" => {
            args.first().and_then(|arg| vector_expr_type(arg, declared))
        }
        Expression::Call { name, .. } if name == "cross" => Some("vec3"),
        Expression::BinaryOp { left, right, .. } => {
            vector_expr_type(left, declared).or_else(|| {
                right
                    .as_deref()
                    .and_then(|value| vector_expr_type(value, declared))
            })
        }
        _ => None,
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
        Expression::LiteralChar(value) => match value {
            '\n' => "'\\n'".to_string(),
            '\r' => "'\\r'".to_string(),
            '\t' => "'\\t'".to_string(),
            '\0' => "'\\0'".to_string(),
            '\\' => "'\\\\'".to_string(),
            '\'' => "'\\''".to_string(),
            value => format!("'{value}'"),
        },
        Expression::LiteralString(s) => s.clone(),
        Expression::LiteralDuration { nanoseconds, .. } => nanoseconds.to_string(),
        Expression::LiteralByteSize { bytes, .. } => bytes.to_string(),
        Expression::LiteralAngle { radians, .. } => format!("{radians:.17}"),
        Expression::VariableReference(name)
            if declared
                .get(name)
                .map(|ty| ty.ends_with("@direct"))
                .unwrap_or(false) =>
        {
            format!("(*{name})")
        }
        Expression::VariableReference(name) => match name.as_str() {
            "PI" => "M_PI".to_string(),
            "TAU" => "(2.0 * M_PI)".to_string(),
            "E" => "M_E".to_string(),
            "EPSILON" => "1e-9".to_string(),
            _ => name.clone(),
        },
        Expression::DirectBorrow(name) => {
            if declared
                .get(name)
                .map(|ty| ty.ends_with("@direct"))
                .unwrap_or(false)
            {
                name.clone()
            } else {
                format!("&{name}")
            }
        }
        Expression::ViewBorrow(name) => {
            if declared
                .get(name)
                .map(|ty| ty.ends_with("@direct"))
                .unwrap_or(false)
            {
                name.clone()
            } else {
                format!("&{name}")
            }
        }
        Expression::Move(name) => {
            let resource_type = declared
                .get(name)
                .map(|ty| normalize_type_token(ty))
                .unwrap_or_default();
            match resource_type.as_str() {
                "Canvas" => format!("sk_canvas_move(&{name})"),
                "Window" => format!("sk_window_move(&{name})"),
                "Interrupt" => format!("sk_interrupt_move(&{name})"),
                _ if channel_elem_from_decl(&resource_type).is_some() => {
                    format!("sk_channel_move(&{name})")
                }
                _ => name.clone(),
            }
        }
        Expression::MemberAccess { base, field } => {
            if base == "my" {
                format!("my->{}", field)
            } else if declared
                .get(base)
                .map(|ty| ty.ends_with("@direct"))
                .unwrap_or(false)
            {
                format!("{}->{}", base, field)
            } else if !declared.contains_key(base)
                && base
                    .chars()
                    .next()
                    .map(|ch| ch.is_ascii_uppercase())
                    .unwrap_or(false)
            {
                format!("{}_{}", base, field)
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
            if name == "color" && args.len() == 4 {
                return format!(
                    "sk_color({}, {}, {}, {})",
                    emit_expr(&args[0], declared),
                    emit_expr(&args[1], declared),
                    emit_expr(&args[2], declared),
                    emit_expr(&args[3], declared)
                );
            }
            if name == "color_hex" && args.len() == 2 {
                return format!(
                    "sk_color_hex({}, {})",
                    emit_expr(&args[0], declared),
                    emit_expr(&args[1], declared)
                );
            }
            if name == "rect" && args.len() == 4 {
                return format!(
                    "sk_rect({}, {}, {}, {})",
                    emit_expr(&args[0], declared),
                    emit_expr(&args[1], declared),
                    emit_expr(&args[2], declared),
                    emit_expr(&args[3], declared)
                );
            }
            if name == "canvas" && args.len() == 2 {
                return format!(
                    "sk_canvas_create({}, {})",
                    emit_expr(&args[0], declared),
                    emit_expr(&args[1], declared)
                );
            }
            if name == "windows.open" && args.len() == 3 {
                return format!(
                    "sk_window_open({}, {}, {})",
                    emit_expr(&args[0], declared),
                    emit_expr(&args[1], declared),
                    emit_expr(&args[2], declared)
                );
            }
            if let Some((receiver_name, method)) = name.split_once('.')
                && let Some(receiver_type) = declared.get(receiver_name)
                && matches!(
                    normalize_type_token(
                        receiver_type
                            .strip_suffix("@direct")
                            .unwrap_or(receiver_type)
                    )
                    .as_str(),
                    "Canvas" | "Window"
                )
            {
                let receiver = if receiver_type.ends_with("@direct") {
                    receiver_name.to_string()
                } else {
                    format!("&{receiver_name}")
                };
                let rendered_args = args
                    .iter()
                    .map(|arg| emit_expr(arg, declared))
                    .collect::<Vec<_>>();
                let function = match method {
                    "clear" => "sk_canvas_clear",
                    "pixel" => "sk_canvas_pixel",
                    "line" => "sk_canvas_line",
                    "rect" => "sk_canvas_rect",
                    "fill_rect" => "sk_canvas_fill_rect",
                    "circle" => "sk_canvas_circle",
                    "fill_circle" => "sk_canvas_fill_circle",
                    "checksum" => "sk_canvas_checksum",
                    "present" => "sk_window_present",
                    "is_open" => "sk_window_is_open",
                    "close" => "sk_window_close",
                    _ => "",
                };
                if !function.is_empty() {
                    if rendered_args.is_empty() {
                        return format!("{function}({receiver})");
                    }
                    return format!("{function}({}, {})", receiver, rendered_args.join(", "));
                }
            }
            if name == "interrupts.periodic" && args.len() == 1 {
                return format!("sk_interrupt_periodic({})", emit_expr(&args[0], declared));
            }
            if let Some((channel_name, method)) = name.split_once('.')
                && let Some(channel_element) = declared
                    .get(channel_name)
                    .and_then(|declared_type| channel_elem_from_decl(declared_type))
            {
                let suffix = channel_type_suffix(channel_element);
                if method == "send" && args.len() == 1 {
                    return format!(
                        "(sk_channel_send_or_panic_{}({}, {}), 0)",
                        suffix,
                        channel_name,
                        emit_expr(&args[0], declared)
                    );
                }
                if method == "try_send" && args.len() == 1 {
                    return format!(
                        "sk_channel_try_send_{}({}, {})",
                        suffix,
                        channel_name,
                        emit_expr(&args[0], declared)
                    );
                }
                if method == "receive" && args.is_empty() {
                    return format!("sk_channel_receive_{}({})", suffix, channel_name);
                }
                if method == "close" && args.is_empty() {
                    return format!("(sk_channel_close({}) ? 0 : 1)", channel_name);
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
                    Builtin::Now if args.is_empty() => {
                        return "sk_time_now()".to_string();
                    }
                    Builtin::Elapsed if args.len() == 1 => {
                        let started_at = emit_expr(&args[0], declared);
                        return format!("sk_time_elapsed({})", started_at);
                    }
                    Builtin::Sleep | Builtin::Delay if args.len() == 1 => {
                        let duration = emit_expr(&args[0], declared);
                        return format!("sk_time_sleep({})", duration);
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
                    Builtin::Dot | Builtin::Distance | Builtin::DistanceSq if args.len() == 2 => {
                        if let Some(vector) = vector_expr_type(&args[0], declared) {
                            let a = emit_expr(&args[0], declared);
                            let b = emit_expr(&args[1], declared);
                            let operation = match builtin {
                                Builtin::Dot => "dot",
                                Builtin::Distance => "distance",
                                Builtin::DistanceSq => "distance_sq",
                                _ => unreachable!(),
                            };
                            return format!("sk_{vector}_{operation}({a}, {b})");
                        }
                    }
                    Builtin::Length | Builtin::LengthSq | Builtin::Normalize if args.len() == 1 => {
                        if let Some(vector) = vector_expr_type(&args[0], declared) {
                            let value = emit_expr(&args[0], declared);
                            let operation = match builtin {
                                Builtin::Length => "length",
                                Builtin::LengthSq => "length_sq",
                                Builtin::Normalize => "normalize",
                                _ => unreachable!(),
                            };
                            return format!("sk_{vector}_{operation}({value})");
                        }
                    }
                    Builtin::Cross if args.len() == 2 => {
                        let a = emit_expr(&args[0], declared);
                        let b = emit_expr(&args[1], declared);
                        return format!("sk_vec3_cross({a}, {b})");
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
        Expression::TimedOut => "sk_channel_timed_out()".to_string(),
        Expression::RunTask { .. } | Expression::WaitTask { .. } => "0".to_string(),
        Expression::BinaryOp { op, left, right } => {
            if op == "neg" {
                let operand = right.as_deref().unwrap_or(left);
                if let Some(vector) = vector_expr_type(operand, declared) {
                    return format!("sk_{vector}_neg({})", emit_expr(operand, declared));
                }
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
                let left_vector = vector_expr_type(left, declared);
                let right_vector = vector_expr_type(r, declared);
                if let Some(vector) = left_vector.or(right_vector) {
                    return match (op.as_str(), left_vector, right_vector) {
                        ("+", Some(_), Some(_)) => format!("sk_{vector}_add({l}, {rr})"),
                        ("-", Some(_), Some(_)) => format!("sk_{vector}_sub({l}, {rr})"),
                        ("*", Some(_), None) => format!("sk_{vector}_scale({l}, {rr})"),
                        ("*", None, Some(_)) => format!("sk_{vector}_scale({rr}, {l})"),
                        ("/", Some(_), None) => format!("sk_{vector}_div({l}, {rr})"),
                        _ => format!("({l} /* unsupported vector operator {op} */ {rr})"),
                    };
                }
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
                    | "dot"
                    | "length"
                    | "length_sq"
                    | "normalize"
                    | "distance"
                    | "distance_sq"
                    | "cross"
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

fn expression_uses_time_call(expr: &Expression) -> bool {
    match expr {
        Expression::Call { name, args } => {
            matches!(name.as_str(), "now" | "elapsed" | "sleep" | "delay")
                || args.iter().any(expression_uses_time_call)
        }
        Expression::RunTask { args, .. } => args.iter().any(expression_uses_time_call),
        Expression::BinaryOp { left, right, .. } => {
            expression_uses_time_call(left)
                || right
                    .as_ref()
                    .map(|value| expression_uses_time_call(value))
                    .unwrap_or(false)
        }
        Expression::Index { base, index } => {
            expression_uses_time_call(base) || expression_uses_time_call(index)
        }
        Expression::ListLiteral(items) => items.iter().any(expression_uses_time_call),
        Expression::StructConstruction { fields } => fields
            .values()
            .any(|value| expression_uses_time_call(value)),
        _ => false,
    }
}

fn stmt_uses_time(stmt: &Statement) -> bool {
    match stmt {
        Statement::VarDecl { value, .. }
        | Statement::Assignment { value, .. }
        | Statement::ExpressionStatement { expr: value, .. }
        | Statement::FieldAssignment { value, .. }
        | Statement::ListPush { value, .. } => expression_uses_time_call(value),
        Statement::ReturnStatement { value, .. } => value
            .as_ref()
            .map(|expression| expression_uses_time_call(expression))
            .unwrap_or(false),
        Statement::IfStatement {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_uses_time_call(condition)
                || then_block.statements.iter().any(stmt_uses_time)
                || else_block
                    .as_ref()
                    .map(|block| block.statements.iter().any(stmt_uses_time))
                    .unwrap_or(false)
        }
        Statement::WhileLoop {
            condition, body, ..
        } => expression_uses_time_call(condition) || body.statements.iter().any(stmt_uses_time),
        Statement::LoopStatement { body, .. } => body.statements.iter().any(stmt_uses_time),
        Statement::ForLoop {
            initialization,
            condition,
            update,
            body,
            ..
        } => {
            initialization
                .as_ref()
                .map(|expression| expression_uses_time_call(expression))
                .unwrap_or(false)
                || condition
                    .as_ref()
                    .map(|expression| expression_uses_time_call(expression))
                    .unwrap_or(false)
                || update
                    .as_ref()
                    .map(|expression| expression_uses_time_call(expression))
                    .unwrap_or(false)
                || body.statements.iter().any(stmt_uses_time)
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
        } => {
            expression_uses_time_call(when_expression)
                || cases.iter().any(|(expressions, block)| {
                    expressions.iter().any(expression_uses_time_call)
                        || block.statements.iter().any(stmt_uses_time)
                })
                || else_block
                    .as_ref()
                    .map(|block| block.statements.iter().any(stmt_uses_time))
                    .unwrap_or(false)
        }
        Statement::DangerAssignOnError { args, on_error, .. }
        | Statement::DangerCallOnError { args, on_error, .. } => {
            args.iter().any(expression_uses_time_call)
                || on_error.statements.iter().any(stmt_uses_time)
        }
        Statement::ListPopOnError { on_error, .. } => {
            on_error.statements.iter().any(stmt_uses_time)
        }
        Statement::FunctionDef { body, .. } => body.statements.iter().any(stmt_uses_time),
        Statement::StructDecl { methods, .. } => methods
            .iter()
            .any(|method| method.body.statements.iter().any(stmt_uses_time)),
        Statement::BlockStatement { statements, .. }
        | Statement::OnErrorBlock { statements, .. } => statements.iter().any(stmt_uses_time),
        _ => false,
    }
}

fn program_uses_time_runtime(program: &Program) -> bool {
    program.statements.iter().any(stmt_uses_time)
}
