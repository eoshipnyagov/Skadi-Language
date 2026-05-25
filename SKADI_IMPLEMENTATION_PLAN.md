# Skadi Implementation Plan

## Plan Date
2026-05-20

## Goal
Reach a working MVP compiler pipeline for a stable subset of Skadi:
- `lex -> parse -> semantic` for representative programs,
- deterministic diagnostics with source locations,
- regression tests for implemented grammar.

## Phase 0 - Baseline Lock (Completed)
Status: completed

Tasks:
- Ensure project compiles with `cargo check`.
- Fix critical compile blockers in lexer/parser contracts.
- Establish project overview and planning docs.

Exit criteria:
- `cargo check` passes.
- Baseline commit created.

## Phase 1 - Parser Core Stabilization
Status: in progress

Tasks:
- Normalize parser entry API in `src/parser/mod.rs`.
- Replace skip-based branches with explicit AST node construction for:
  - function declarations,
  - assignments,
  - `if` / `while` / `loop`,
  - `for in`,
  - `when` skeleton with case capture.
- Add structured parse errors (message + token location).

Exit criteria:
- Parser returns deterministic results for valid/invalid minimal programs.
- No panic paths in normal parse flow.

## Phase 2 - Expression Engine (Pratt Parser)
Status: in progress

Tasks:
- Implement precedence table from `Skadi_design.txt`.
- Add prefix/infix parsing for arithmetic/comparison/logical operators.
- Support grouped expressions and variable references.

Exit criteria:
- Precedence-correct AST for expressions.
- Tests for at least 15 precedence/associativity scenarios.

## Phase 3 - Semantic Analysis v1
Status: in progress

Tasks:
- Scope-aware symbol table validation.
- Checks:
  - use-before-definition,
  - duplicate declarations in same scope,
  - self-reference in first assignment,
  - basic assignment compatibility.
- Produce user-facing diagnostics with line/column.

Exit criteria:
- Semantic pass catches core scope/type errors on fixture set.
- Diagnostic format stable across runs.

## Phase 4 - Integration and Fixtures
Status: planned

Tasks:
- Add fixture-based tests for:
  - small unit snippets,
  - `example_meteostation.txt` (integration sample).
- Add pass/fail expectation files.
- Add CI-friendly test command.

Exit criteria:
- `cargo test` validates MVP grammar slice.
- Integration fixture is part of regression suite.

## Phase 5 - Language Feature Expansion
Status: planned

Tasks:
- Incrementally implement remaining spec features:
  - `danger fn` + `on error`,
  - structs/methods + `my`,
  - selected stdlib-aware semantics.
- Keep each feature behind tests before merging.

Exit criteria:
- Feature checklist mapped to spec sections with coverage status.

## Phase 6 - Language Design Review (Near-Term)
Status: planned

Tasks:
- Revisit v1 language scope and explicitly reduce non-essential features for MVP.
- Resolve syntax/model overlap (one canonical style per feature in v1).
- Reconfirm semantics for memory model (`allow drop`, chunk budgeting) before deeper implementation.
- Freeze a reduced "Skadi Core v1" subset and map compiler milestones strictly to that subset.
- Align all syntax decisions with `docs/SKADI_STYLE_PRINCIPLES.md`.
- Add TODO track for human-readable output formatting API (avoid low-level `%...` formatting noise in everyday code).
  - candidate direction: readable formatter helper for mixed numeric/text output in v1.x.

## Toolchain TODO - Target Compilation
Status: planned

Tasks:
- Research and document cross-target build flows for Skadi -> C -> target binary:
  - AVR (embedded),
  - ESP family (Xtensa/RISC-V depending on chip),
  - ARM targets (including common embedded profiles),
  - Linux targets (x86_64/ARM where practical).
- Define compiler backend/toolchain matrix:
  - required C toolchains per target,
  - minimal build commands,
  - expected runtime constraints for generated C code.
- Add first feasibility checklist:
  - "builds C successfully",
  - "links target binary",
  - "runs hello-world style smoke for target environment/emulator where possible".

Exit criteria:
- Written design decision record for v1 scope cuts and kept features.
- Updated grammar/spec section for the reduced core subset.
- Implementation plan updated to prioritize only frozen core features.

## Phase 7 - List/Text v1 Realization
Status: in progress

Tasks:
- Freeze accepted syntax/typing in RFCs (`docs/RFC_LIST.md`, `docs/RFC_TEXT.md`).
- Parser + AST:
  - support `new <Type> List <name> = ...`
  - support list literals `[a, b, c]`
  - support `Text` indexing syntax shape in parser pipeline
- Semantic:
  - enforce list declaration/type rules from RFC
  - validate `len(List)` / `len(Text)` and index argument types
  - validate `push`/`pop` signatures and `danger` usage contract
- Codegen/runtime (C):
  - define `List`/`Text` runtime ABI
  - lower list/text operations to runtime calls
  - implement minimal runtime helpers for v1 operations
- Diagnostics:
  - add stable semantic/runtime-facing codes for List/Text errors
  - cover expected failure modes in tests

Exit criteria:
- Parser accepts canonical List/Text v1 examples from RFC docs.
- Semantic pass validates all agreed v1 rules.
- C backend can build and run at least one e2e list/text fixture.

## Risk Register
1. Contract drift between lexer token kinds and parser expectations.
Mitigation: freeze shared enums in `common_types.rs`, change only with tests.

2. Parser complexity growth without expression engine.
Mitigation: prioritize Pratt parser before expanding statement grammar.

3. Regressions due to scaffold code paths.
Mitigation: convert placeholders to explicit errors where behavior is not implemented.

4. Syntax drift away from readability goals.
Mitigation: enforce `docs/SKADI_STYLE_PRINCIPLES.md` as review baseline.

## Working Rules
- Every new grammar feature must include at least one positive and one negative test.
- Prefer small commits per phase task.
- Keep `Skadi_design.txt` as the normative grammar reference.
- Keep syntax choices aligned with `docs/SKADI_STYLE_PRINCIPLES.md`.

