# RFC: `List` v1 Baseline

Status: Accepted (v1 baseline)
Date: 2026-05-22
Owner: Skadi core

## 1. Final syntax (v1)

- Typed declaration uses type-before-name style:

  - `new i32 List xs = [1, 2, 3]`
  - `new Float List values = []`
- `List(T)` is not user-facing syntax in v1.
- Indexing:

  - read: `x = xs[i]`
  - write: `xs[i] = 42`

## 2. Final API surface (v1)

- `len(xs)` -> returns list length (`Int`)
- `xs.push(v)` -> append one value of list element type
- `xs.pop()` -> returns element, must be used with `on error` when empty:

  - `v = xs.pop() on error { ... }`

## 3. Type rules (v1)

- Element type is mandatory in declaration (`new <Type> List <name> = ...`).
- Literal elements must be assignable to declared element type.
- `xs[i]` requires integer index type.
- Assigned value for `xs[i] = v` must be assignable to element type.
- `push(v)` requires `v` assignable to element type.

## 4. Error behavior (v1)

- Empty `pop()` is a recoverable runtime error (danger flow).
- Out-of-bounds indexing is a recoverable runtime error (danger flow).
- Runtime growth failure is a recoverable runtime error (danger flow).

Exact `ErrorCode` naming is finalized in runtime/codegen phase.

## 5. Explicit non-goals (v1)

- `map/filter/reduce`
- list comprehensions
- immutable/persistent list variants
- advanced allocator tuning surface
- mutation-safety policy for modifying a list during active iteration (e.g., `pop` in loop body)
  - candidate v2 decisions:

  - `A)` forbid structural mutation of iterated list
  - `B)` allow with defined dynamic-length semantics

## 6. Canonical examples

```skadi
new i32 List xs = [1, 2, 3]
new i32 first = xs[0]
xs[1] = 10
new Int n = len(xs)
xs.push(99)
new i32 last = xs.pop() on error {
    last = 0
}
```

