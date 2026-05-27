# Skadi Documentation Policy

Date: 2026-05-27
Status: Active

## 1. Language workflow

- Working mode (day-to-day): maintain docs in English only under `docs/`.
- Release mode: produce full Russian duplicate set under `docs/RU/`.
- Russian docs in `docs/RU/` are release snapshots, not daily-edit sources.

## 2. Source of truth

- Canonical editable source is English in `docs/`.
- RU files must include, at top:
  - `Source: docs/<EN_FILE>.md`
  - `Synced for release: <version/date>`

## 3. Release rule (mandatory)

Before any release tag/candidate:
1. Ensure all release-scoped docs from `docs/` have RU copies in `docs/RU/`.
2. Verify no stale RU files (timestamp/version in header must match release).
3. Run link check for both trees (`docs/` and `docs/RU/`).

## 4. File naming convention (new and migrated docs)

- Use lowercase kebab-case names for active docs.
- Prefer semantic prefixes:
  - `guide-*` for user guides,
  - `reference-*` for reference docs,
  - `contract-*` for frozen behavior contracts,
  - `rfc-*` for proposals,
  - `matrix-*` for status/coverage matrices.

Examples:
- `quick-start.md`
- `guide-install-new-machine.md`
- `reference-cli.md`
- `reference-language.md`
- `contract-on-error-v1.md`

## 5. Migration strategy for existing names

- Do not rename all files in one large change.
- Rename in small batches and update links in the same commit.
- Keep temporary compatibility stubs only when needed for external links.

## 6. Ownership

- Every docs-impacting PR must update:
  - relevant file in `docs/`,
  - `docs/DOCS_INDEX.md` when navigation changes,
  - RU release snapshot only during release preparation.
