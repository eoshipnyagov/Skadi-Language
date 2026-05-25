# RFC: `Text` v1 Baseline

Status: Accepted (v1 baseline)
Date: 2026-05-22
Owner: Skadi core

## 1. Final syntax (v1)

- Declaration:
  - `new Text t = "hello"`
- Length:
  - `n = len(t)`
- Indexing:
  - `ch = t[i]`
- Contains:
  - `ok = contains(t, "sub")`
- Find:
  - `idx = find(t, "sub")`
- Slice:
  - `part = slice(t, start, end)`

## 2. Type rules (v1)

- String literal can initialize `Text` directly.
- `len(Text)` returns `Int`.
- Index for `t[i]` must be integer type.
- `t[i]` result type in v1 is `char`.
- `contains(Text, Text)` returns `bool`.
- `find(Text, Text)` returns `Int` index of first match, `-1` if not found.
- `slice(Text, Int, Int)` returns `Text`.

## 3. Error behavior (v1)

- `slice` bounds are normalized:
  - `start < 0` -> `0`
  - `end < start` -> `start`
  - `start/end > len(text)` -> `len(text)`
- `slice` returns an allocated null-terminated text fragment.
- `Text` operations in v1 use byte-oriented UTF-8 behavior (not grapheme-aware).

## 4. Deferred to v2

- concat operators/helpers for `Text`
- normalization and locale-aware transforms
- regex-style APIs
- advanced grapheme-aware operations

## 5. Canonical examples

```skadi
new Text t = "weather"
new Int n = len(t)
new char c = t[0]
new bool has = contains(t, "ath")
new Int idx = find(t, "the")
new Text part = slice(t, 1, 4)
```

