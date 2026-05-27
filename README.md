# Skadi Language (Skadi -> C Compiler Prototype)

Skadi is a human-readable systems language experiment with a practical compiler pipeline:

`Skadi source -> lexer -> parser -> semantic -> C transpiler -> native C compiler`

Current repository status: active `v1` stabilization (tests, diagnostics, CLI UX, cross-platform CI).

## Quick Start

Prerequisites:
- Rust toolchain (`cargo`, `rustc`)
- C compiler in `PATH` (`gcc` / `clang` / `cc`, on Windows also `cl`)

Run checks:

```bash
cargo test -q
cargo clippy --all-targets --all-features
```

Run CLI doctor:

```bash
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor
```

Create and run a demo project:

```bash
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- new console demo
cd demo
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- check
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- build
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- run
```

## Key Documentation

- Getting started:
  - [Quick Start](docs/QUICK_START.md)
  - [Install on a New Machine](docs/INSTALL_NEW_MACHINE.md)
- Language and syntax:
  - [Skadi Language Reference (RU)](docs/SKADI_LANGUAGE_REFERENCE_RU.md)
  - [Syntax Status](docs/SKADI_SYNTAX_STATUS.md)
  - [Canonical Syntax Matrix v1 (RU)](docs/SYNTAX_CANONICAL_MATRIX_V1_RU.md)
- Project architecture:
  - [Project Tech Reference (RU)](docs/SKADI_PROJECT_TECH_REFERENCE_RU.md)
  - [Skadi->C Scope](docs/SKADI_TO_C_SCOPE.md)
- Quality and release:
  - [Test Coverage Matrix](docs/TEST_COVERAGE_MATRIX.md)
  - [Token/Construct Coverage Matrix](docs/TOKEN_CONSTRUCT_COVERAGE_MATRIX.md)
  - [v1 Blockers Matrix (RU)](docs/V1_BLOCKERS_MATRIX_RU.md)
  - [Diagnostics Codes Reference](docs/DIAGNOSTIC_CODES_REFERENCE.md)
  - [v1 Release Contract (RU)](docs/V1_RELEASE_CONTRACT_RU.md)
  - [Release Notes v1.0.0-rc1 (RU)](docs/RELEASE_NOTES_V1_RC1_RU.md)
- Full docs map:
  - [Docs Index](docs/DOCS_INDEX.md)

## Repository Layout

- `src/` — compiler core (`lexer`, `parser`, `semantic_analysis`, `codegen`)
- `tools/skadi-cli/` — CLI manager (`doctor`, `new`, `check`, `build`, `run`, `clean`, `tui`)
- `tests/` — unit/integration/e2e tests for pipeline/codegen
- `docs/` — language/runtime contracts, status matrices, RFCs
- `benchmarks/` — showcase programs

## Current v1 Contract (short)

- Module imports: only `import "./relative_path.skd"` is in v1 scope.
- Diagnostics are stabilized by stage/code families:
  - parser: `SC-PARSE-*`
  - semantic: `SC-SEM-*`
  - module/import: `SC-MOD-001`
  - native compile: `SC-CGEN-001`
- CLI pipeline wrappers use normalized stage messages (`SC-LEX-000`, `SC-PARSE-000`, `SC-SEM-000`).

## License

This project is licensed under the [MIT License](LICENSE).
