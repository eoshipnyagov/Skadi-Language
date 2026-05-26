# Changelog

## [v1.0.0-rc1] - 2026-05-27

### Added
- Statement-level support for `break`, `continue`, `pass`, `i++`, `i--` across parser/semantic/codegen.
- Expanded `codegen_e2e` feature-mix and stress scenarios (text/list/struct/import graph/when chains/danger recovery).
- Import-graph stress e2e for deep chain and wide diamond module layouts.
- Diagnostic contract tests for module/parse/semantic/codegen pipeline stages.

### Changed
- Codegen for struct-list runtime:
  - fixed struct typedef emission order before list helpers,
  - fixed fail-soft default return for struct list indexing (`(Type){0}`).
- CI workflow split into:
  - required `test-matrix`,
  - required `codegen-e2e`,
  - optional `sanitizer-optional`.
- Pipeline diagnostics normalized to `code + stage + hint`.

### Docs
- Synced coverage and blocker matrices with actual test state.
- Added diagnostic codes reference updates for stage-wrapper ownership.
- Added `docs/V1_RELEASE_CONTRACT_RU.md` (draft for approval).
- Updated `docs/CLI_USAGE.md` and `docs/QUICK_START.md` to current CLI behavior.
