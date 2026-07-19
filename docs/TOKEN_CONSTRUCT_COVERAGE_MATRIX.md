# Token/Construct Coverage Matrix (current `develop`)

Date: 2026-07-19
Purpose: traceability across stable `v1.1` and experimental `v1.2` systems surface.

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
| `returns` | Y | Y | Y | Y | Y | canonical typed return syntax |
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
| `Memory` / `memory(size)` | Y | Y | Y | Y | Y | experimental fixed-capacity region runtime |
| `place in` / trailing `on error` | Y | Y | Y | Y | Y | placement and overflow recovery covered |
| `memory.clear()` | Y | Y | Y | Y | Y | use-after-clear and active-region rules covered |
| `Task` / `Task(T)` / `run` | Y | Y | Y | Y | Y | experimental native Win32/pthread runtime |
| `wait` / `stop` / `stopping` | Y | Y | Y | Y | Y | path-sensitive lifecycle and cooperative stop covered |
| `Channel(T)` / `channel(N)` | Y | Y | Y | Y | Y | bounded blocking FIFO runtime |
| `send` / `receive` | Y | Y | Y | Y | Y | value-safe payload and backpressure covered |
| `Time` / `Duration` | P (type identifiers) | Y | Y | Y | Y | experimental nominal value types |
| `5ms` / `2s` / `3min` | P (number + adjacent unit) | Y | Y | Y | Y | integer and overflow-checked literals |
| `now` / `elapsed` | P (identifiers) | Y | Y | Y | Y | monotonic clock runtime |
| `sleep` / `delay` | P (identifiers) | Y | Y | Y | Y | blocking host runtime |

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
- Memory frontend/runtime/examples: `tests/memory_model_frontend.rs`, `tests/memory_model_examples.rs`
- Task/Channel frontend/runtime/TSan: `tests/task_model_frontend.rs`, `tests/task_model_runtime.rs`, `tests/task_model_sanitizer.rs`
- Compile-checked small examples: `tests/language_programs.rs`, `examples/language/`
- Showcase systems coverage: `tests/showcase_programs.rs`, `benchmarks/bench_11_task_channel_pipeline.skd`, `benchmarks/bench_12_systems_pipeline.skd`
- Time/Duration frontend/runtime: `tests/time_model.rs`, `tests/codegen_e2e.rs`, `benchmarks/bench_13_time_budget.skd`
- Multi-file/import graph and mutation-like negative e2e: `tools/skadi-cli/src/pipeline.rs` tests

## 4. Synchronization rules

1. Keep experimental `fixed/const/direct/allow drop` forms explicitly separated from the stable surface.
2. Keep this matrix synchronized with the [Test Coverage Matrix](test-coverage.md) after each feature merge.
