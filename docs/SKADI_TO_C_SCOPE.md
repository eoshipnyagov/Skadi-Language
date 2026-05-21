# Skadi to C Scope (v1)

## Status
Active near-term backend target.
Date: 2026-05-21

## Goal
Produce readable, deterministic C output for `Skadi Core v1` to avoid implementing a full backend/runtime from scratch at this stage.

## Included in v1
- top-level function declarations (`fn`, `danger fn`)
- assignment statements
- arithmetic/comparison/logical expressions currently supported by parser
- `if / else`, `while`, `loop`
- `for item in collection` (transitional lowering)

## Transitional Mappings
- first assignment to a variable in a scope: emitted as declaration (`int name = ...;`)
- subsequent assignments: emitted as re-assignment (`name = ...;`)
- `loop { ... }` -> `while (1) { ... }`
- `for item in collection { ... }` -> temporary C-style loop placeholder until list runtime model is defined

## Deferred / Not Yet Lowered
- full `when` case lowering
- `on error` semantics and danger call binding
- `on event`/`on interrupt` runtime semantics
- memory model features (`allow drop`, chunk budgeting)
- full type mapping beyond baseline numeric/inferred placeholders

## Output Principles
- generated C should be stable (same input -> same output)
- generated C should be readable for debugging
- unsupported constructs should be represented with explicit TODO comments in C output

## Validation
- smoke tests assert generation of major structures
- parser+semantic must pass before C generation in pipeline
