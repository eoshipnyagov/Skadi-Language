# Skadi Docs Index (Source of Truth Map)

Date: 2026-05-27
Purpose: one-page map of which document answers which question.

## 1. Start Here

- `docs/QUICK_START.md` — cross-platform bootstrap and first commands.
- `docs/INSTALL_NEW_MACHINE.md` — full setup and troubleshooting on a fresh machine.
- `README_PROJECT_OVERVIEW.md` — short project overview and current status.

## 2. Language Behavior (Current Snapshot)

- `docs/SKADI_SYNTAX_STATUS.md` — what syntax works right now.
- `docs/SKADI_LANGUAGE_REFERENCE_RU.md` — practical RU language reference.
- `docs/SYNTAX_CANONICAL_MATRIX_V1_RU.md` — canonical style for v1 writing.

## 3. Contracts (Frozen/Normative for v1)

- `docs/ON_ERROR_V1_MATRIX_RU.md` — `on error` allow/deny matrix.
- `docs/TEXT_V1_CONTRACT_RU.md` — `Text` contract.
- `docs/C_RUNTIME_MEMORY_CONTRACT_V1_RU.md` — C runtime memory rules.

## 4. Architecture and Scope

- `docs/SKADI_PROJECT_TECH_REFERENCE_RU.md` — project architecture map.
- `docs/SKADI_TO_C_SCOPE.md` — explicit Skadi->C scope.
- `docs/SKADI_CLI_RFC.md` — CLI scope/status and command model.

## 5. RFC / Backlog

- `docs/RFC_LIST.md` — `List` baseline.
- `docs/RFC_TEXT.md` — `Text` baseline.
- `docs/RFC_MATH_VECTOR_CORE.md` — draft for math/vector expansion.
- `docs/MATH_VECTOR_CORE_BACKLOG_1X_RU.md` — deferred math/vector items.

## 6. Quality Gates

- `docs/TEST_COVERAGE_MATRIX.md` — test coverage and codegen regression guardrails.
- `docs/TOKEN_CONSTRUCT_COVERAGE_MATRIX.md` — explicit token/construct-to-tests traceability matrix.
- `docs/V1_BLOCKERS_MATRIX_RU.md` — release blockers and status.
- `docs/DIAGNOSTICS_STYLE.md` — canonical diagnostics format.
- `docs/DIAGNOSTIC_CODES_REFERENCE.md` — canonical ownership map for diagnostic code families.
- `docs/V1_RELEASE_CONTRACT_RU.md` — release freeze contract (draft/approval gate).

## 7. Release Notes

- `CHANGELOG.md` — release-level change log.
- `docs/RELEASE_NOTES_V1_RC1_RU.md` — candidate release notes for `v1.0.0-rc1`.

## 8. Style

- `docs/SKADI_STYLE_PRINCIPLES.md` — design principles.
- `docs/SKADI_STYLE_GUIDE_V1.md` — coding style for examples/showcase.

## 9. Showcase

- `docs/SHOWCASE_PROGRAMS.md` — small programs used as confidence suite.

## 10. Conflict Resolution Rule

If two docs conflict:
1. `SKADI_SYNTAX_STATUS.md` wins for implemented behavior.
2. `*_CONTRACT_*` docs win for frozen v1 runtime/error rules.
3. RFC docs are intent/proposals and do not override implemented behavior.
