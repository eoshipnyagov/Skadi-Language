# RFC Draft: `List` Type

Status: Draft
Date: 2026-05-22
Owner: TBD

## 1. Problem
`List` is planned as a core high-level type, but the language currently has no finalized contract for:
- syntax
- operations
- runtime shape
- error behavior
- interaction with chunk memory model

Without this contract, compiler/runtime work risks rework.

## 2. Goals for v1
- provide one minimal, coherent `List` model
- support predictable lowering to C
- support core flow use cases:
  - declaration/init
  - indexing
  - `len`
  - append/remove (minimal mutating API)
  - `for item in list`

## 3. Non-Goals for v1
- advanced functional API (`map/filter/reduce`)
- persistent/immutable lists
- concurrent lock-free list structures
- allocator tuning APIs exposed to users

## 4. Open Syntax Questions
1. Type notation:
- option A: `List(Int)`
- option B: `List[Int]`

2. Construction:
- option A: `new List(Int) xs = []`
- option B: `xs = List(Int)`

3. Method surface:
- `xs.push(v)`, `xs.pop()`, `xs.len`, `xs[i]`

## 5. Proposed Minimal Runtime Shape (C)
Per-element-type specialization, e.g.:
- `SkadiListInt { int64_t* data; size_t len; size_t cap; }`
- `SkadiListFloat { double* data; size_t len; size_t cap; }`
- `SkadiListBool { bool* data; size_t len; size_t cap; }`

## 6. Error Semantics
Questions to decide:
1. out-of-bounds index -> `danger` error vs runtime abort
2. `pop()` on empty -> `danger` error vs sentinel value
3. allocation growth failure -> `danger` error code

Recommendation: explicit `danger` codes for all recoverable runtime failures.

## 7. Memory Model Integration
Must define:
- where list buffer lives relative to chunk
- growth policy
- what happens on chunk pressure
- how `allow drop` may interact with list storage

## 8. Compiler Work Items After RFC Approval
1. parser support for final chosen list syntax
2. semantic type representation for generic element type
3. type checks for indexing/method calls
4. codegen for list runtime helpers
5. tests: parser + semantic + codegen + e2e

## 9. Decision Checklist
- [ ] syntax finalized
- [ ] core API finalized
- [ ] error semantics finalized
- [ ] C runtime ABI finalized
- [ ] chunk-memory interaction finalized
