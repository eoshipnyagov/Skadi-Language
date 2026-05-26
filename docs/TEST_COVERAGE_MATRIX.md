# Test Coverage Matrix (Skadi v1 Prototype)

Date: 2026-05-25
Owner: Skadi core

This file tracks test coverage for language elements in the current Rust prototype.

## 1. Covered now

- Lexing diagnostics and tokenization
  - `tests/lexer_smoke.rs`
- Parsing core statements/expressions
  - `tests/parser_smoke.rs`
  - includes: `new`, typed `new`, `if/else`, `while`, `for in`, `iterate ... as ...`,
    function defs, `danger fn`, `return`, `return error`, `when/is/else`, `label`,
    `struct` shape, `on interrupt`, list literals, list push/pop-on-error, indexing, calls
- Semantic validation
  - `tests/semantic_smoke.rs`
  - includes: type mismatch, scope/redeclaration, use-before-def, call arity/types,
    `danger` + `on error` binding checks, `ErrorCode` rules, list typing, text typing,
    `for/iterate` item inference, text builtin checks (`len/contains/find/slice`),
    negative checks for `on error` on non-danger builtins (`read/write/fs.list`),
    style-canonical warnings (`iterate` preference, `Bool/Char` preference)
- Code generation shape checks
  - `tests/codegen_smoke.rs`
  - includes C lowering of control flow, `when`, list runtime calls, text runtime calls,
    danger-call lowering shape, typed declarations
- Integration pipeline tests
  - `tests/language_programs.rs`
  - end-to-end through lex -> parse -> semantic -> C generation for multi-feature programs
- C compiler e2e tests
  - `tests/codegen_e2e.rs`
  - C output compiles and produced binaries run for representative programs
  - includes edge scenarios for `Text` bounds/empty-needle and `List` + `when` flow
  - includes UTF-8 text byte-semantics smoke scenario
  - includes sanitizer-backed stress scenario (`ASan/UBSan`) when compiler supports flags
  - memory contract tie-in: validates no sanitizer-detected crashes/UB for current runtime allocation model
  - includes feature-mix scenarios (struct+method + iterate/when + i++/i--, and io/fs branching mixes)
  - includes negative compile e2e guard for known semantic/codegen mismatch shape
- Edge matrix conformance set
  - `tests/edge_matrix.rs`
  - includes:
    - numeric List coverage across `i/u/f` families (`8/16/32/64`) and `bool`
    - `Path List` lowering to text runtime helpers
    - extreme text index/slice shapes
    - UTF-8 text contract shape (byte-oriented `len/index/slice` lowering)
    - negative builtin argument/type checks (`fs.join`, `write`, `args`)
    - struct-list iteration + method calls
    - `danger` + `on error` + explicit `ErrorCode` flow

## 2. Partially covered / pending deep checks

- Runtime out-of-range policy for indexing
  - frozen for v1 as fail-soft (`List` index -> `0`, `Text` index -> `'\0'`)
  - codegen contract tests cover helper behavior shape
- `on interrupt` / `on event` runtime semantics
  - parse-level coverage exists, runtime binding remains TODO
- Concurrency primitives (`run`, `wait`, `Link`) and embedded APIs
  - not implemented in current transpiler/runtime slice

## 3. Policy for new features

For each newly implemented feature, add:

1. parser test (`parser_smoke`)
2. semantic positive + negative tests (`semantic_smoke`)
3. codegen shape check (`codegen_smoke`)
4. at least one integration scenario (`language_programs` or `codegen_e2e`)

## 4. Codegen Invariants (golden-lite)

Critical lowering markers are asserted directly in tests (stable fragments, not full-file snapshots):
- `when -> if / else if` chain markers (`__when_tmp_*`).
- `danger fn` + `on error` lowering markers (`fn(..., *out)` + `if (call(...) != 0)`).
- runtime hooks for `List/Text/fs/io`.
- statement-only inc/dec lowering (`i += 1;` / `i -= 1;`).

