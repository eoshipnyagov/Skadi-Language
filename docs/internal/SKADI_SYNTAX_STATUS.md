# Skadi Syntax Status (Current Snapshot)

Date: 2026-05-27
Purpose: single source of truth for what syntax works in this repository.

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
- `local fn name(...) { ... }` - `Stable`
- `danger fn name(...) ... { ... }` - `Stable`
- typed params: `fn add(Int a, Float b)` - `Stable`
- canonical typed return: `fn add(...) returns Int` - `Stable`
- legacy typed return without `returns`: `fn add(...) Int` - `Partial`
  - currently allowed for simple scalar types with style warning
  - struct return without explicit `returns` is semantic error
- function calls in expressions: `x = add(a, b)` - `Stable`
- signature checks (arity/types) for calls - `Stable`

## Control Flow
- `if / else if / else` - `Stable`
- `while` - `Stable`
- `loop` - `Stable`
- `break` - `Stable` (only inside loops)
- `continue` - `Stable` (only inside loops)
- `pass` - `Stable` (no-op statement)
- `i++` / `i--` - `Stable` (statement-only, not allowed inside expressions)
- `for item in collection` - `Partial`
  - lowering assumes list runtime shape: `collection.len` + `collection.data[i]`
- `iterate collection as item` - `Partial` (alias of `for`)
- `when / is / else` - `Stable`
  - lowers to `if / else if / else`
  - `is a, b` supported

## Error Flow
- `x = danger_call(...) on error { ... }` - `Stable`
- `danger_call(...) on error { ... }` - `Stable`
- `on error` allowed only for calls to `danger fn` - `Stable`

## Structs and Visibility
- `struct Name { ... }` - `Stable`
- `local struct Name { ... }` - `Stable`
- `label Name { ... }` - `Stable`
- `local label Name { ... }` - `Stable`
- `my.field` in methods - `Stable`
- `hide` fields and restricted external access - `Stable`
- shadowing forbidden - `Stable`

## Imports / Modules (V1.1)
- `import "./relative_path.skd"` - `Stable`
  - recursive, cycle-safe, deduplicated, deterministic merge order
- `local fn/struct/label` are hidden across imports - `Stable`
- direct-import-only visibility enforced for entry file - `Stable`
  - transitive symbol access from entry fails with `SC-MOD-003`
- deterministic import public-symbol collision error - `Stable`
  - collisions fail with `SC-MOD-002`
- Not supported in current wave:
  - `import module_name`
  - alias form (`import "./x.skd" as x`)

## Intentionally Deferred
- `direct` params
- `returns struct { ... }`
- events/interrupt runtime semantics
- chunk memory features (`allow drop`, budgets)
- test DSL keywords

## Design Note
Keyword naming may change later. This file tracks the implemented parser contract now, not a final language freeze.
