# Skadi Syntax Status (Current Snapshot)

Date: 2026-05-25
Purpose: single source of truth for "what syntax actually works right now" in this repository.

## Stability Levels
- `Stable`: implemented, tested, expected to work.
- `Partial`: implemented with constraints / transitional behavior.
- `Planned`: in spec, not implemented here yet.

## Core Statements
- `new x = expr` - `Stable`
- `new Type x = expr` - `Stable` (scalar types: `Int`, `Float`, `bool`)
- `x = expr` - `Stable` (requires prior declaration)
- `return expr` - `Stable`
- `return` - `Stable` (special behavior in `danger fn`)
- `return error Code` - `Stable` (only in `danger fn`, with `label ErrorCode`)

## Functions
- `fn name(...) { ... }` - `Stable`
- `danger fn name(...) ... { ... }` - `Stable`
- typed params: `fn add(Int a, Float b)` - `Stable`
- typed return: `fn add(...) Int` - `Stable`
- function calls in expressions: `x = add(a, b)` - `Stable`
- signature checks (arity/types) for calls - `Stable`

## Control Flow
- `if / else if / else` - `Stable`
- `while` - `Stable`
- `loop` - `Stable`
- `for item in collection` - `Partial`
  - lowering assumes list runtime shape: `collection.len` + `collection.data[i]`
  - element type is lowered from declared list element type in codegen
  - style note: supported for familiarity/compatibility; `iterate ... as ...` is preferred for showcase style.
- `iterate collection as item` - `Partial` (alias)
  - parsed as an alias of `for item in collection`
  - currently lowers through the same `ForLoop` path
- `when / is / else` - `Stable` (MVP)
  - lowers to `if / else if / else`
  - `is a, b` supported
  - type compatibility between `when` expression and `is` cases is validated

## Error Flow
- `danger` call with handler:
  - `x = danger_call(...) on error { ... }` - `Stable`
  - `danger_call(...) on error { ... }` - `Stable`
- `on error` allowed only for calls to `danger fn` - `Stable`
- `danger fn` C ABI:
  - returns status `int`
  - optional out-param for value
  - success path: `return 0`
  - explicit error path: `return error X` -> `return ErrorCode_X`

## Labels
- `label Name { A B C }` parsing - `Stable`
- `label ErrorCode` semantic contract - `Stable`
  - first variant must be `Ok`
  - `return error X` requires `X` in `ErrorCode`

## Types (Current)
- Checked scalars: `Int`, `Float`, `bool`
- implicit widening: `Int -> Float` allowed
- bool conditions in `if/while` are required
- `List` and `Text` are intentionally **not finalized** yet

## Indexing Contract (Frozen for v1)
- `xs[i]` for `List` is `fail-soft` in runtime:
  - out-of-range returns default zero value (`0`) for the element C type.
- `t[i]` for `Text` is `fail-soft` in runtime:
  - out-of-range returns `'\0'`.
- `on error` is **not** attached to index access in `v1`.
- Possible stricter `danger`-style indexing is deferred to `v2+` discussion.

## Type Naming Canonical Style (v1)
- Low-level fixed-width numeric types stay lowercase:
  - `i8`, `i16`, `i32`, `i64`
  - `u8`, `u16`, `u32`, `u64`
  - `f32`, `f64`
- Readability-oriented/common user-facing types stay capitalized:
  - `Int`, `Float`, `Bool`, `Char`, `Text`, `Path`, `List`, `Vec2`, `Vec3`, `Vec4`
- Transitional note:
  - `bool` and `char` remain accepted as compatibility aliases.
  - For style docs/examples, prefer readability-first naming (`Bool`, `Char`) and keep fixed-width primitives lowercase.

## Intentionally Deferred
- `direct` params
- `returns struct { ... }`
- `local fn`
- imports/modules
- struct fields/methods (`my.field`)
- events/interrupt runtime semantics
- chunk memory features (`allow drop`, budgets)
- test DSL keywords

## Design Note
Keyword naming may change later. This file tracks the implemented parser contract now, not a final language freeze.
