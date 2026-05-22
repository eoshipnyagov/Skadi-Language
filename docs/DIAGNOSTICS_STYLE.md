# Diagnostics Style Guide

This document defines the canonical user-facing diagnostic format for Scadi compiler stages.

## Canonical format

`<Kind> error at line <L>, col <C>[, index <I>]: [<CODE>] <message>`

Where:
- `<Kind>` is one of: `Lex`, `Parse`, `Semantic`.
- `<CODE>` is optional but recommended (`SC-LEX-001`, `SC-PARSE-003`, `SC-SEM-020`).
- `line`/`col` are 1-based source coordinates when available.
- `index` is optional token index (currently used by parser entry diagnostics).
- `<message>` must be concise, actionable, and describe the exact failure.

When location data is not available:

`<Kind> error: [<CODE>] <message>`

## Message conventions

- Start with lowercase unless using a language symbol/name (`ErrorCode`, `Int`, etc.).
- Prefer domain-specific phrasing:
  - `use-before-definition`
  - `type mismatch in assignment`
  - `unknown function 'foo'`
- Include related symbol names in single quotes.
- Avoid stack-trace style or internal debug noise in user messages.

## Examples

- `Lex error at line 3, col 12: [SC-LEX-001] unexpected character '@'`
- `Parse error at line 5, col 1, index 14: [SC-PARSE-003] expected '{' after 'if' condition.`
- `Semantic error at line 9, col 7: [SC-SEM-020] type mismatch in assignment to 'x': cannot assign Bool to Int.`

## Semantic code map (current)

- `SC-SEM-010` redeclaration in scope
- `SC-SEM-011` invalid self-referential initialization
- `SC-SEM-012` use-before-definition
- `SC-SEM-020` type mismatch
- `SC-SEM-030` unknown function
- `SC-SEM-031` argument count mismatch
- `SC-SEM-032` argument type mismatch
- `SC-SEM-040` invalid semantic context
- `SC-SEM-050` return-path/return-form rule
- `SC-SEM-051` `ErrorCode` label/variant rule
- `SC-SEM-900` internal semantic consistency error
