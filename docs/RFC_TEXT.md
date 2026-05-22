# RFC: `Text` v1 Baseline

Status: Accepted (v1 baseline)
Date: 2026-05-22
Owner: Scadi core

## 1. Final syntax (v1)

- Declaration:
  - `new Text t = "hello"`
- Length:
  - `n = len(t)`
- Indexing:
  - `ch = t[i]`

## 2. Type rules (v1)

- String literal can initialize `Text` directly.
- `len(Text)` returns `Int`.
- Index for `t[i]` must be integer type.
- `t[i]` result type in v1 is `char`.

## 3. Error behavior (v1)

- Out-of-range index access is a recoverable runtime error (danger flow).
- Exact error code names are finalized with runtime integration.

## 4. Deferred to v2

- `slice(a, b)`
- concat operators/helpers for `Text`
- normalization and locale-aware transforms
- regex-style APIs
- advanced grapheme-aware operations

## 5. Canonical examples

```skadi
new Text t = "weather"
new Int n = len(t)
new char c = t[0]
```
