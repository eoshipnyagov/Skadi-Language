# Skadi Compiler Project Overview

## 1. Purpose
This repository contains a Rust prototype of the Skadi compiler.

Primary goal of the current stage:
- make the compiler pipeline stable and testable,
- lock core contracts (tokens and AST),
- move from a partial prototype to a working MVP compiler flow.

Language source of truth:
- `Scadi_design.txt` (Skadi Language Specification v1.1)

## 2. Current Status (as of 2026-05-20)
Implemented and working:
- project builds with `cargo check`,
- lexer module structure is in place and usable,
- parser supports core top-level constructs and expression parsing via Pratt parser,
- semantic analysis includes baseline scope checks (`use-before-definition`, redeclaration, self-reference init),
- regression tests for parser/semantic smoke scenarios are in place.

Still in progress:
- parser and semantic stages are not yet feature-complete vs full spec,
- diagnostics are not yet fully standardized around source spans,
- some legacy helper methods remain unused and need cleanup.

## 3. Repository Structure
Top-level:
- `Cargo.toml` - Rust package config
- `README_PROJECT_OVERVIEW.md` - project snapshot and architecture overview
- `SCADI_IMPLEMENTATION_PLAN.md` - implementation roadmap and phase tracking
- `Scadi_design.txt` - Skadi v1.1 specification
- `example_meteostation.txt` - integration-style sample program
- `docs/SKADI_STYLE_PRINCIPLES.md` - accepted style/design baseline for syntax decisions

Source tree (`src/`):
- `lib.rs` - module wiring for compiler components
- `main.rs` - executable pipeline runner (`lex -> parse -> semantic`)
- `common_types.rs` - shared token contracts and common lexical types
- `lexer/mod.rs` - lexer module entry and re-exports
- `lexer/core.rs` - lexer implementation (`Lexer`, `lex`)
- `lexer/structures.rs` - lexer-related shared structures/errors
- `ast_nodes.rs` - AST node types and scope manager scaffold
- `parser/mod.rs` - parser orchestration over token stream
- `parser/statements.rs` - statement/declaration parsing logic
- `parser/expressions.rs` - Pratt parser for expressions
- `semantic_analysis.rs` - semantic analysis pass (current baseline checks)

Tests (`tests/`):
- `parser_smoke.rs` - parser integration smoke tests
- `semantic_smoke.rs` - semantic integration smoke tests
- `fixtures/` - fixture snippets used by tests

## 4. Compilation Pipeline Target
Target architecture (MVP -> full):
1. Source file loading
2. Lexing (`source -> Vec<Token>`)
3. Parsing (`Vec<Token> -> AST Program`)
4. Semantic analysis (`AST -> validated AST / diagnostics`)
5. Future: code generation backend (IR / WASM / native)

Current executable (`main.rs`) runs stages 2-4 for the sample source.

## 5. Development Principles
- Keep `Scadi_design.txt` as the functional baseline.
- Keep style decisions aligned with `docs/SKADI_STYLE_PRINCIPLES.md`.
- Ship in vertical slices (end-to-end capability for a small subset).
- Add tests whenever behavior is promoted from scaffold to implemented logic.

## 6. Short-Term Priorities
1. Complete parser coverage for core statements and error forms.
2. Strengthen semantic checks for function and block scopes.
3. Normalize diagnostics with consistent source locations.
4. Expand fixture-based regression coverage.

## 7. How To Run
Validation build:
```bash
cargo check
```

Run test suite:
```bash
cargo test
```

Run executable pipeline:
```bash
cargo run
```

## 8. Related Planning Files
- `SCADI_IMPLEMENTATION_PLAN.md`
- `docs/SKADI_STYLE_PRINCIPLES.md`

## 9. Near-Term Design Revisit
In the near term, language design should be revisited before broad feature expansion.
Focus:
- narrow v1 scope to a smaller "Skadi Core" subset,
- keep one canonical syntax path per feature,
- postpone high-complexity runtime/memory features until core compiler stages are stable.
