# Skadi Docs Index

Date: 2026-05-27
Goal: single map of active docs with minimal overlap.

## Start Here

- `docs/QUICK_START.md` - 5-minute bootstrap.
- `docs/INSTALL_NEW_MACHINE.md` - full setup and troubleshooting on a fresh machine.
- `docs/CLI_USAGE.md` - command reference for `skadi`.
- `README_PROJECT_OVERVIEW.md` - project snapshot.

## Language (Current Behavior)

- `docs/SKADI_SYNTAX_STATUS.md` - implemented syntax status.
- `docs/SKADI_LANGUAGE_REFERENCE_RU.md` - practical language reference (RU).
- `docs/SYNTAX_CANONICAL_MATRIX_V1_RU.md` - canonical style matrix.

## Runtime and Error Contracts (v1)

- `docs/ON_ERROR_V1_MATRIX_RU.md`
- `docs/TEXT_V1_CONTRACT_RU.md`
- `docs/C_RUNTIME_MEMORY_CONTRACT_V1_RU.md`

## Architecture and Scope

- `docs/SKADI_PROJECT_TECH_REFERENCE_RU.md`
- `docs/SKADI_TO_C_SCOPE.md`

## Quality and Release Gates

- `docs/TEST_COVERAGE_MATRIX.md`
- `docs/TOKEN_CONSTRUCT_COVERAGE_MATRIX.md`
- `docs/V1_BLOCKERS_MATRIX_RU.md`
- `docs/V1_RELEASE_CONTRACT_RU.md`
- `docs/KNOWN_ISSUES.md`
- `docs/DIAGNOSTICS_STYLE.md`
- `docs/DIAGNOSTIC_CODES_REFERENCE.md`

## RFC and Backlog

- `docs/RFC_LIST.md`
- `docs/RFC_TEXT.md`
- `docs/RFC_MATH_VECTOR_CORE.md`
- `docs/MATH_VECTOR_CORE_BACKLOG_1X_RU.md`

## Showcase and Style

- `docs/SHOWCASE_PROGRAMS.md`
- `docs/SKADI_STYLE_PRINCIPLES.md`
- `docs/SKADI_STYLE_GUIDE_V1.md`

## Design Baseline

- `docs/design/Skadi_design_v1_1.txt`

## Archived Docs

- `docs/old/SKADI_CLI_RFC.md`
- `docs/old/RELEASE_NOTES_V1_RC1_RU.md`

## Conflict Rule

1. `SKADI_SYNTAX_STATUS.md` wins for implemented behavior.
2. `*_CONTRACT_*` docs win for frozen v1 runtime/error rules.
3. RFC docs are intent only and do not override implementation.
