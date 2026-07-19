from __future__ import annotations

from pathlib import Path
import shutil


ROOT = Path(__file__).resolve().parent.parent
DOCS_RU = ROOT / "docs"
DOCS_EN = ROOT / "docs-en"
BUILD = ROOT / ".docs-build"


ROUTE_MAP = [
    ("user/getting-started", "SKADI_GETTING_STARTED_RU.md"),
    ("user/cli-quick-start", "SKADI_CLI_QUICK_START_RU.md"),
    ("user/cli-reference", "SKADI_CLI_REFERENCE_RU.md"),
    ("user/language-reference", "SKADI_LANGUAGE_REFERENCE_RU.md"),
    ("user/concurrency", "SKADI_CONCURRENCY_GUIDE_RU.md"),
    ("user/syntax-status", "SKADI_SYNTAX_STATUS.md"),
    ("user/ai-guide", "SKADI_FOR_AI_RU.md"),
    ("user/showcases", "SHOWCASE_PROGRAMS.md"),
    ("internal/project-tech-reference", "SKADI_PROJECT_TECH_REFERENCE_RU.md"),
    ("internal/to-c-scope", "SKADI_TO_C_SCOPE.md"),
    ("internal/test-coverage", "TEST_COVERAGE_MATRIX.md"),
    ("internal/diagnostics-style", "DIAGNOSTICS_STYLE.md"),
    ("internal/implementation-plan", "docs/SKADI_IMPLEMENTATION_PLAN_RU.md"),
    ("internal/v1-1-plan", "SKADI_V1_1_PLAN_RU.md"),
    ("internal/v1-2-plan", "SKADI_V1_2_PLAN_RU.md"),
    ("internal/style-principles", "SKADI_STYLE_PRINCIPLES.md"),
    ("internal/v1-non-goals", "SKADI_V1_NON_GOALS_RU.md"),
    ("internal/style-guide-v1", "SKADI_STYLE_GUIDE_V1.md"),
    ("internal/syntax-canonical-matrix", "SYNTAX_CANONICAL_MATRIX_V1_RU.md"),
    ("internal/text-contract-v1", "TEXT_V1_CONTRACT_RU.md"),
    ("internal/on-error-v1", "ON_ERROR_V1_MATRIX_RU.md"),
    ("internal/c-runtime-memory-contract-v1", "C_RUNTIME_MEMORY_CONTRACT_V1_RU.md"),
    ("internal/v1-blockers", "V1_BLOCKERS_MATRIX_RU.md"),
    ("internal/math-vector-backlog", "MATH_VECTOR_CORE_BACKLOG_1X_RU.md"),
    ("internal/docs-site-and-i18n", "DOCS_SITE_AND_I18N_RU.md"),
    ("internal/memory-model-draft", "SKADI_MEMORY_MODEL_DRAFT_RU.md"),
    ("internal/memory-model-mvp", "SKADI_MEMORY_MODEL_MVP_CONTRACT_RU.md"),
    ("internal/memory-model-examples", "SKADI_MEMORY_MODEL_EXAMPLES_RU.md"),
    ("internal/task-model-draft", "SKADI_TASK_MODEL_DRAFT_RU.md"),
    ("internal/task-model-mvp", "SKADI_TASK_MODEL_MVP_CONTRACT_RU.md"),
    ("internal/task-runtime-mvp-design", "SKADI_TASK_RUNTIME_MVP_DESIGN_RU.md"),
    ("internal/visual-core-draft", "SKADI_VISUAL_CORE_DRAFT_RU.md"),
    ("internal/visual-core-mvp", "SKADI_VISUAL_CORE_MVP_CONTRACT_RU.md"),
    ("internal/systems-additions-draft", "SKADI_SYSTEMS_ADDITIONS_DRAFT_RU.md"),
    ("internal/systems-additions-mvp", "SKADI_SYSTEMS_ADDITIONS_MVP_CONTRACT_RU.md"),
    ("internal/rfc-text", "RFC_TEXT.md"),
    ("internal/rfc-list", "RFC_LIST.md"),
    ("internal/rfc-math-vector-core", "RFC_MATH_VECTOR_CORE.md"),
]


def read_text(path: Path) -> str:
    for encoding in ("utf-8", "utf-8-sig", "cp1251"):
        try:
            return path.read_text(encoding=encoding)
        except UnicodeDecodeError:
            continue
    raise UnicodeDecodeError("unknown", b"", 0, 1, f"unable to decode {path}")


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def title_from_route(route: str) -> str:
    return route.split("/")[-1].replace("-", " ").title()


def find_ru_source(rel: str) -> Path:
    docs_candidate = (DOCS_RU / rel).resolve()
    if docs_candidate.exists():
        return docs_candidate
    candidate = (ROOT / rel).resolve()
    return candidate


def find_en_source(ru_rel: str) -> Path | None:
    base_name = Path(ru_rel).name
    candidate = DOCS_EN / base_name.replace("_RU", "_EN")
    return candidate if candidate.exists() else None


def make_home_ru() -> str:
    return """# Skadi Docs

Это HTML-слой документации Skadi.

## Разделы

- [Пользовательские документы](user/index.md)
- [Внутренние документы разработки](internal/index.md)

## Как пользоваться

- если вы хотите писать программы на Skadi, начните с пользовательского раздела;
- если вы развиваете язык, компилятор или runtime-контракты, идите во внутренний раздел.
"""


def make_home_en() -> str:
    return """# Skadi Docs

This is the HTML documentation layer for Skadi.

## Sections

- [User Docs](user/index.en.md)
- [Internal Docs](internal/index.en.md)

## How to use it

- if you want to write Skadi programs, start with the user docs;
- if you are developing the language, compiler, or runtime contracts, use the internal docs.
"""


def make_user_index_ru() -> str:
    return """# Пользовательские документы

Этот раздел предназначен для тех, кто пишет программы на Skadi и работает с
`skadi-cli` и `skadi-cli tui`.

Стабильная пользовательская база соответствует `v1.1`; текущая разработка
`v1.2` добавляет experimental systems tracks поверх неё.

## Основные страницы

- [Начало работы](getting-started.md)
- [Быстрый старт CLI](cli-quick-start.md)
- [Справочник CLI/TUI](cli-reference.md)
- [Справочник языка](language-reference.md)
- [Многопоточность](concurrency.md)
- [Статус синтаксиса](syntax-status.md)
- [Showcase-программы](showcases.md)
"""


def make_user_index_en() -> str:
    return """# User Docs

This section is for people writing programs in Skadi and using `skadi-cli` and
`skadi-cli tui`.

The stable user-facing base corresponds to `v1.1`; current `v1.2` development
adds experimental systems tracks on top of it.

## Main pages

- [Getting Started](getting-started.en.md)
- [CLI Quick Start](cli-quick-start.en.md)
- [CLI/TUI Reference](cli-reference.en.md)
- [Language Reference](language-reference.en.md)
- [Concurrency](concurrency.en.md)
- [Syntax Status](syntax-status.en.md)
- [Showcase Programs](showcases.en.md)
"""


def make_internal_index_ru() -> str:
    return """# Внутренние документы разработки

Этот раздел предназначен для разработки языка, компилятора, диагностики,
контрактов, текущих experimental tracks и будущих архитектурных направлений.

## Основные группы

- текущее состояние компилятора;
- контракты `v1`;
- планы и блокеры;
- текущие experimental tracks `v1.2`;
- будущие языковые треки;
- устройство сайта документации и локализации.
"""


def make_internal_index_en() -> str:
    return """# Internal Development Docs

This section is for language, compiler, diagnostics, contracts, current
experimental tracks, and future-track development work.

## Main groups

- current compiler state;
- `v1` contracts;
- plans and blockers;
- current `v1.2` experimental tracks;
- future language tracks;
- documentation site and localization setup.
"""


def make_extra_css() -> str:
    return """.md-container {
  display: flex;
  flex-direction: column;
  min-height: calc(100vh - 3rem);
}

.md-typeset {
  font-size: 0.88rem;
  line-height: 1.68;
}

.md-typeset h1,
.md-typeset h2,
.md-typeset h3,
.md-typeset h4 {
  line-height: 1.25;
  letter-spacing: -0.01em;
  margin-top: 1.3em;
}

.md-typeset p,
.md-typeset ul,
.md-typeset ol,
.md-typeset blockquote,
.md-typeset table {
  margin-top: 0.85em;
  margin-bottom: 0.85em;
}

.md-typeset code,
.md-typeset pre code {
  font-size: 0.94em;
}

.md-typeset pre {
  border-radius: 0.75rem;
}

.md-main {
  flex: 1 0 auto;
}

.md-main__inner {
  min-height: 100%;
}

.md-content {
  padding-bottom: 2rem;
}

.md-content__inner {
  padding-top: 1rem;
}

.md-footer {
  display: none;
}

.md-sidebar__scrollwrap {
  overflow-y: auto;
  max-height: calc(100vh - 4rem);
  padding-bottom: 100px;
}

.md-sidebar__inner::after {
  content: "";
  display: block;
  height: 100px;
}
"""


def make_en_placeholder(route: str, ru_rel: str) -> str:
    title = title_from_route(route)
    return f"""# {title}

> English translation is not ready yet.

This page is currently maintained in Russian first.

## Source

- Russian source file: `{ru_rel}`
- Russian page: switch to `Русский` in the language selector

## Translation policy

- Russian is the current source of truth for most documentation.
- English pages are added incrementally.
"""


def sync_clean_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def main() -> None:
    sync_clean_dir(BUILD)

    write_text(BUILD / "index.md", make_home_ru())
    write_text(BUILD / "index.en.md", make_home_en())
    write_text(BUILD / "user" / "index.md", make_user_index_ru())
    write_text(BUILD / "user" / "index.en.md", make_user_index_en())
    write_text(BUILD / "internal" / "index.md", make_internal_index_ru())
    write_text(BUILD / "internal" / "index.en.md", make_internal_index_en())
    write_text(BUILD / "assets" / "extra.css", make_extra_css())

    for route, ru_rel in ROUTE_MAP:
        ru_source = find_ru_source(ru_rel)
        ru_target = BUILD / f"{route}.md"
        write_text(ru_target, read_text(ru_source))

        en_source = find_en_source(ru_rel)
        en_target = BUILD / f"{route}.en.md"
        if en_source is not None:
            write_text(en_target, read_text(en_source))
        else:
            write_text(en_target, make_en_placeholder(route, ru_rel))

    print(f"Synced docs site sources into {BUILD}")


if __name__ == "__main__":
    main()
