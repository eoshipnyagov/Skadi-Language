# Test Coverage Matrix (Skadi v1 Prototype)

Date: 2026-05-27
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
  - total: `26` e2e scenarios (including feature-mix, negative pipeline checks, and stress cases)
  - includes feature-mix scenarios for:
    - `when + find/len + Text`
    - `danger/on error` in loops
    - `List + while + indexing`
    - `fs.list + fs.is_dir + when`
    - `break/continue/pass` + `i++/i--` loop-control path
  - includes UTF-8 text byte-semantics smoke scenario
  - includes large-shape stress scenarios:
    - big `when` chain
    - deep/wide import-graph e2e in CLI pipeline tests
    - large allocation loops for `List`/`Struct List` and intensive `Text` ops
  - includes sanitizer-backed stress scenario (`ASan/UBSan`) when compiler supports flags
  - memory contract tie-in: validates no sanitizer-detected crashes/UB for current runtime allocation model
- CI gate structure (GitHub Actions):
  - required: `test-matrix` (non-e2e + CLI), `codegen-e2e` (full `codegen_e2e` suite)
  - optional: `sanitizer-optional` (ASan/UBSan scenario; explicit log via `--nocapture`)
- CLI pipeline mutation-like negative e2e tests
  - `tools/skadi-cli/src/pipeline.rs` test module
  - verifies deterministic failure stage + diagnostic code for:
    - parse errors (`SC-PARSE-*`)
    - semantic contract errors (`SC-SEM-*`)
    - native compile failures (`SC-CGEN-001`)
  - verifies import graph contract diagnostics (`SC-MOD-001`)
  - verifies deterministic import symbol collision diagnostics (`SC-MOD-002`)
  - verifies direct-import-only visibility diagnostics (`SC-MOD-003`)
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
- Full token/construct traceability matrix
  - TODO: explicit `keyword/lexeme -> lexer/parser/semantic/codegen/e2e` mapping file
  - goal: no implicit coverage assumptions before v1 freeze

## 3. Policy for new features

For each newly implemented feature, add:

1. parser test (`parser_smoke`)
2. semantic positive + negative tests (`semantic_smoke`)
3. codegen shape check (`codegen_smoke`)
4. at least one integration scenario (`language_programs` or `codegen_e2e`)

## 4. Scope/Visibility v1.1 checklist

Source contract: `docs/internal/SCOPE_VISIBILITY_V1_1.md`

Must-cover cases:
1. Negative: shadowing in nested block fails.
2. Positive: `my.field` vs local name resolution.
3. Hidden fields: direct access fails, own-method access succeeds.
4. `local fn/struct/label` visibility from importer.
5. Import name collision deterministic failure.
6. Qualified `module.symbol` for `fn`/`struct`/`label`.
7. Negative: no transitive import visibility.

