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
- parser scaffolding exists for key constructs (`fn`, `for`, `when`, basic control flow),
- shared contracts (`Token`, `TokenKind`, AST skeleton) are centralized.

Still in progress:
- parser is largely scaffold-level and does not fully parse expressions/blocks by spec,
- semantic analysis is not yet integrated as a robust stage,
- warning count is high (dead code / placeholders), which is expected for this phase.

## 3. Repository Structure
Top-level:
- `Cargo.toml` - Rust package config
- `README_PROJECT_OVERVIEW.md` - project snapshot and architecture overview
- `Scadi_design.txt` - Skadi v1.1 specification
- `example_meteostation.txt` - integration-style sample program
- `cargo_output.txt` - historical compilation diagnostics snapshot

Source tree (`src/`):
- `lib.rs` - module wiring for compiler components
- `main.rs` - current executable entry point (lexing run)
- `common_types.rs` - shared token contracts and common lexical types
- `lexer_utils.rs` - lexer helper utilities
- `lexer/core.rs` - lexer implementation (`Lexer`, `lex`)
- `lexer/structures.rs` - lexer-related shared structures/errors
- `ast_nodes.rs` - AST node types and scope manager scaffold
- `parsing_logic.rs` - parser helper functions for language constructs
- `parser.rs` - parser orchestration over token stream
- `semantic_analysis.rs` - semantic analysis placeholder module
- `location.rs` - source location utilities scaffold

## 4. Compilation Pipeline Target
Target architecture (MVP -> full):
1. Source file loading
2. Lexing (`source -> Vec<Token>`)
3. Parsing (`Vec<Token> -> AST Program`)
4. Semantic analysis (`AST -> validated AST / diagnostics`)
5. Future: code generation backend (IR / WASM / native)

Current executable (`main.rs`) runs Stage 2 directly (lexing) and serves as a stable smoke-test entrypoint.

## 5. Development Principles
- Keep `Scadi_design.txt` as the single functional spec baseline.
- Ship in vertical slices (end-to-end capability for a small subset).
- Avoid broad refactors while parser/semantic contracts are still moving.
- Add tests whenever a grammar feature is promoted from scaffold to implemented behavior.

## 6. Short-Term Priorities
1. Stabilize parser contracts and expression parsing strategy.
2. Introduce Pratt parser for precedence-correct expressions.
3. Integrate semantic checks for scope and base type consistency.
4. Expand test corpus from mini-samples + `example_meteostation.txt`.

## 7. How To Run
Validation build:
```bash
cargo check
```

Run current executable:
```bash
cargo run
```

Expected current runtime behavior:
- reads `example_meteostation.txt`,
- runs lexer,
- prints tokenization success/failure.

## 8. Related Planning File
Detailed implementation roadmap is tracked in:
- `SCADI_IMPLEMENTATION_PLAN.md`

## 9. Near-Term Design Revisit
In the near term, the language design should be revisited before broad feature expansion.
Focus of that review:
- narrow v1 scope to a smaller \"Skadi Core\" subset,
- reduce overlapping syntax/semantics to one canonical path per feature,
- postpone high-complexity runtime/memory features until core compiler stages are stable.
