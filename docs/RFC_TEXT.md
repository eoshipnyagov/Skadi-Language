# RFC Draft: `Text` Type

Status: Draft
Date: 2026-05-22
Owner: TBD

## 1. Problem
`Text` is intended as a stronger string type (likely UI/indexing oriented), but its contract is undefined:
- relation to `String`
- indexing semantics
- mutation model
- complexity guarantees
- encoding boundaries

## 2. Goals for v1
- define clear separation between `String` and `Text`
- define predictable indexing behavior
- keep runtime/codegen feasible for C backend

## 3. Non-Goals for v1
- full Unicode normalization pipeline
- advanced locale-dependent transforms
- rich rope/rope-tree editing structures

## 4. Key Design Questions
1. What does index mean?
- byte index
- codepoint index
- grapheme cluster index

2. Is `Text` mutable?
- immutable value type
- mutable buffer-like type

3. Required operations in v1:
- length
- slice
- indexing
- concat
- conversion to/from `String`

## 5. Suggested Direction (MVP)
- `String`: compact UTF-8 storage-first type
- `Text`: indexing-friendly abstraction with explicit complexity tradeoff
- require explicit conversion between them where semantics differ

## 6. Error Semantics
Decide:
1. out-of-range text index
2. invalid slice boundaries
3. conversion failures (if any)

Recommendation: align with `danger` + `ErrorCode` flow for recoverable errors.

## 7. Runtime/Codegen Notes
Need to decide whether `Text` lowers to:
- plain UTF-8 buffer + helper functions
- or richer struct with index map/cache

## 8. Compiler Work Items After RFC Approval
1. syntax and type rules for `Text`
2. semantic checks for indexing/slicing
3. codegen runtime helpers
4. tests (parser/semantic/codegen/e2e)

## 9. Decision Checklist
- [ ] semantics of indexing finalized
- [ ] `String` vs `Text` boundary finalized
- [ ] minimal API finalized
- [ ] error behavior finalized
- [ ] runtime representation finalized
