# Skadi to C Scope (v1)

## Status
Active near-term backend target.
Date: 2026-05-21

## Goal
Produce readable, deterministic C output for `Skadi Core v1` to avoid implementing a full backend/runtime from scratch at this stage.

## Included in v1 (implemented)
- top-level function declarations (`fn`, `danger fn`)
- variable declarations via `new`:
  - `new name = expr`
  - `new Type name = expr`
- assignment statements (`name = expr`) for already defined variables
- arithmetic/comparison/logical expressions currently supported by parser
- `if / else`, `while`, `loop`
- `return`
- `for item in collection` (transitional lowering)
- baseline Skadi->C type mapping for declarations:
  - `Int`/`i64` -> `int64_t`
  - `Float`/`f64` -> `double`
  - `bool` -> `bool`

## Transitional Mappings
- `loop { ... }` -> `while (1) { ... }`
- `for item in collection { ... }` -> temporary C-style loop placeholder until list runtime model is defined
- unsupported constructs are emitted as explicit `TODO(v1)` comments in C

## Priority Roadmap
1. Function signatures and typing
- parse/validate typed params (`fn add(Int a, Int b)`)
- parse/validate typed returns (`fn add(...) Int`)
- emit mapped C signatures

2. Type checking minimum viable layer
- enforce assignment compatibility for core scalar types
- define explicit behavior for mixed arithmetic (`Int + Float`)
- add explicit cast strategy in AST/codegen where needed

3. Danger/on-error lowering
- represent danger call result in C as status/value pair
- lower `x = danger_call(...) on error { ... }`
- lower bare `danger_call(...) on error { ... }`

4. Control-flow completeness
- real `when/is/else` lowering (if-chain or switch where applicable)
- canonical else-if lowering in AST/codegen path

5. Runtime-oriented syntax (post-MVP)
- `run/wait/delay`
- `Link(T)` send/receive/signal semantics
- `on interrupt`/`on event` runtime binding model

## Deferred / Not Yet Implemented
- full `returns struct { ... }` lowering
- `direct` params semantics
- `local fn` module visibility enforcement
- struct field access (`my.field`) and method lowering
- imports and module path resolution
- memory model features (`allow drop`, chunk budgeting)
- test DSL (`test`, `check`)

## Output Principles
- generated C should be stable (same input -> same output)
- generated C should be readable for debugging
- unsupported constructs should be represented with explicit TODO comments in C output

## Validation
- smoke tests assert generation of major structures
- parser+semantic must pass before C generation in pipeline
- e2e test builds generated C with `clang/gcc/cc` when available
