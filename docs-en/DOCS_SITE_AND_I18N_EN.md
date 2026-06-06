# Docs Site and Localization

This page describes the current technical setup for the Skadi HTML documentation
site and the RU/EN documentation flow.

## Current source of truth

- Russian Markdown files in `docs/` are the primary source of truth.
- English source pages live in `docs-en/`.
- If an English page is missing, the docs-site sync step generates a placeholder
  page instead of pretending the page is fully translated.

## Site generation flow

The current stack is:

- `MkDocs`
- `Material for MkDocs`
- `mkdocs-static-i18n`

The docs site is generated from Markdown, not edited directly:

1. source files are read from `docs/` and `docs-en/`;
2. `scripts/sync_docs_site.py` assembles the docs tree into `.docs-build/`;
3. `mkdocs build` or `mkdocs serve` produces the final HTML output in `site/`.

## Translation policy

- Russian-first authoring is intentional.
- English pages are added incrementally where polished translations are useful.
- Machine translation can be used as a draft, but published English pages should
  be reviewed before they become a source page in `docs-en/`.

## Current practical scope

The first pages worth keeping translated are user-facing pages such as:

- getting started;
- CLI/TUI usage;
- showcase programs;
- selected docs-site workflow notes.

Internal drafts and contracts can continue to fall back to placeholders until
they need real English readers.
