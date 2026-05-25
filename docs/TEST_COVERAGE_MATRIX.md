# Test Coverage Matrix (Skadi v1 Prototype)

Date: 2026-05-25
Owner: Scadi core

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
    negative checks for `on error` on non-danger builtins (`read/write/fs.list`)
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
  - includes sanitizer-backed stress scenario (`ASan/UBSan`) when compiler supports flags
- Edge matrix conformance set
  - `tests/edge_matrix.rs`
  - includes:
    - numeric List coverage across `i/u/f` families (`8/16/32/64`) and `bool`
    - `Path List` lowering to text runtime helpers
    - extreme text index/slice shapes
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
