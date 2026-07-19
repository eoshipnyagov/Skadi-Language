# Token/Construct Coverage Matrix (v1 Tracking)

Date: 2026-07-19
Purpose: explicit traceability for confidence before v1 freeze.

Legend:
- `Y` = covered by current tests
- `P` = partial (implemented with constraints or indirect checks)
- `N` = not covered / not implemented

## 1. Keyword / Lexeme Matrix

| Lexeme / Token intent | Lexer | Parser | Semantic | Codegen | E2E | Notes |
|---|---|---|---|---|---|---|
| `fn` | Y | Y | Y | Y | Y | core function path covered in smoke+e2e |
| `danger fn` | Y | Y | Y | Y | Y | includes `return error` flow |
| `struct` | Y | Y | P | P | P | lowering works for current subset; advanced cases pending |
| `label` | Y | Y | Y | Y | Y | `ErrorCode` contract covered |
| `if` / `else` | Y | Y | Y | Y | Y | includes nested/branch checks |
| `when` / `is` / `else` | Y | Y | Y | Y | Y | includes marker/invariant and e2e scenarios |
| `for ... in ...` | Y | Y | Y | P | P | style-supported; lowering tied to list runtime shape |
| `iterate ... as ...` | P (`iterate` as identifier lexeme) | Y | Y | P | P | parser alias path covered |
| `while` | Y | Y | Y | Y | Y | covered broadly |
| `loop` | Y | Y | Y | Y | P | e2e less dense than while/for |
| `return` | Y | Y | Y | Y | Y | includes empty return in danger fn |
| `return error` | Y | Y | Y | Y | Y | requires `label ErrorCode` |
| `new` | Y | Y | Y | Y | Y | typed/untyped paths covered |
| `my` | Y | Y | P | P | P | struct method subset covered |
| `on error` | P (`on` token + parse pattern) | Y | Y | Y | Y | danger/list-pop contracts covered |
| `on interrupt` | P (`on` + `interrupt`) | Y | P | N | N | parse/semantic placeholder only |
| `fixed` / `const` | Y | P | P | N | N | tokenized; non-core execution path |
| `hide` | Y | Y | Y | P | P | hidden-field access checks implemented; broader struct-lowering depth is ongoing |
| `local` | Y | Y | Y | P | P | local visibility enforced in import pipeline via symbol isolation |
| `direct` | Y | P | N | N | N | deferred semantics |
| `allow drop` | P (`allow` tokenized) | P | N | N | N | chunk-memory design deferred |
| `import "./... .skd"` | N (resolved in CLI pipeline pre-lex) | N (pre-merged) | N (pre-merged) | N (pre-merged) | Y | covered in `tools/skadi-cli` tests |
| `import module_name` / alias | N | N | N | N | Y (negative) | deterministic diagnostic `[SC-MOD-001]` |
| import public symbol collision | N | N | N | N | Y (negative) | deterministic diagnostics `[SC-MOD-002]` |
| direct-import-only visibility | N | N | N | N | Y (negative) | deterministic diagnostics `[SC-MOD-003]` |
| `and` / `or` / `xor` / `not` | Y | Y | Y | Y | P | operator paths covered; dense combo e2e can grow |
| `div` / `mod` | Y | Y | Y | Y | P | covered in parser/semantic/smoke, moderate e2e density |
| `true` / `false` | Y | Y | Y | Y | Y | bool pipelines covered |

## 2. Operator / Form Matrix

| Construct | Lexer | Parser | Semantic | Codegen | E2E | Notes |
|---|---|---|---|---|---|---|
| Assignment `=` | Y | Y | Y | Y | Y | base path |
| Compound assign `+= -= *= /=` | Y | P | P | P | N | partial parser/codegen usage |
| Comparison `== != > < >= <=` | Y | Y | Y | Y | Y | includes text compare lowering |
| Arithmetic `+ - * / % ^` | Y | Y | Y | Y | Y | broad coverage |
| Indexing `xs[i]`, `t[i]` | Y | Y | Y | Y | Y | fail-soft contract tested |
| Function call `f(...)` | Y | Y | Y | Y | Y | includes builtin and user fn |
| List literal `[ ... ]` | Y | Y | Y | Y | Y | multiple scalar families + struct list |
| Struct literal `{field = ...}` | Y | Y | Y | P | P | stable subset covered |
| `i++` / `i--` statement-only | P (lex as operators) | Y | Y | Y | Y | statement-only behavior enforced and covered end-to-end |
| `break` / `continue` / `pass` | P (lex as identifiers today) | Y | Y | Y | Y | loop-scope semantics and lowering covered end-to-end |

## 3. Coverage Sources

- Lexer: `tests/lexer_smoke.rs`
- Parser: `tests/parser_smoke.rs`, `tests/parser_negative.rs`
- Semantic: `tests/semantic_smoke.rs`
- Codegen shape: `tests/codegen_smoke.rs`, `tests/edge_matrix.rs`, `tests/language_programs.rs`
- End-to-end C compile/run: `tests/codegen_e2e.rs`
- Multi-file/import graph and mutation-like negative e2e: `tools/skadi-cli/src/pipeline.rs` tests

## 4. Pre-freeze TODO (explicit)

1. Keep experimental `fixed/const/direct/allow drop` forms explicitly separated from the stable surface.
2. Keep this matrix synchronized with the [Test Coverage Matrix](test-coverage.md) after each feature merge.
