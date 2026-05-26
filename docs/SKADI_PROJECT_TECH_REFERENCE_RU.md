# Skadi: Техническая Документация Проекта (RU)

Дата актуальности: 2026-05-26

## 1. Архитектура в общих чертах

Проект разделен на два основных слоя:
- `src/` — компиляторное ядро (lexer/parser/semantic/codegen),
- `tools/skadi-cli/` — пользовательский менеджер сборки и проектов (`new/check/build/run/doctor/...`).

Базовый путь обработки:
1. Чтение `.skd` и разрешение импортов.
2. Лексический анализ (`Token`).
3. Парсинг в AST.
4. Семантическая проверка.
5. Генерация C.
6. Внешняя компиляция C в бинарник (через выбранный C-компилятор).

## 2. Структура репозитория

- `Cargo.toml` — workspace/зависимости Rust.
- `src/` — библиотека компилятора.
- `tests/` — unit/integration/e2e тесты.
- `tools/skadi-cli/` — отдельный crate CLI-менеджера.
- `docs/` — спецификации, RFC, матрицы покрытия и блокеров.
- `scripts/` — вспомогательные smoke/automation скрипты.
- `benchmarks/` — короткие программы для showcase/regression.
- `old/` — архив устаревших материалов.

## 3. Модули компиляторного ядра (`src/`)

### `src/lib.rs`
Реэкспорт модулей ядра как библиотечного API.

### `src/main.rs`
Легковесный CLI раннер базового pipeline для отладки ядра.

### `src/common_types.rs`
Базовые токены и общие типы, которыми пользуются lexer/parser.

### `src/diagnostics.rs`
Единый формат диагностик (lex/parse/semantic) с кодом и локацией.

### `src/builtins.rs`
Таблица builtin-функций и их сигнатур для семантики/codegen.

### `src/ast_nodes.rs`
Определения AST-узлов (`Program`, `Statement`, `Expression`, и т.д.).

### `src/lexer/`
- `mod.rs` — экспорт API лексера,
- `structures.rs` — структуры и ошибки лексера,
- `core.rs` — токенизация исходника.

### `src/parser/`
- `mod.rs` — orchestration парсинга,
- `expressions.rs` — Pratt-parser выражений,
- `statements.rs` — разбор деклараций/стейтментов.

### `src/semantic_analysis.rs`
Типизация, правила контекстов, проверка `danger/on error`, валидации control-flow и т.д.

### `src/codegen/`
- `mod.rs` — экспорт codegen API,
- `c.rs` — lowering AST в C + runtime helpers (`List`, `Text`, часть FS/I/O glue).

## 4. `tools/skadi-cli` (практическая точка входа)

`tools/skadi-cli/src/main.rs` — роутинг команд.

Ключевые модули:
- `commands/` — реализации подкоманд (`new`, `init`, `check`, `build`, `run`, `clean`, `doctor`, `examples`).
- `project.rs` — загрузка/валидация `skadi.toml`, проектный контекст.
- `pipeline.rs` — запуск `Skadi -> C -> native` и обработка ошибок пайплайна.
- `targets.rs` — host/target и выбор toolchain.
- `templates.rs` — встроенные шаблоны проектов.

## 5. Текущий V1-контракт (что считается стабильным)

- Расширение языка: `.skd`.
- Multi-file импорт: только `import "./relative_path.skd"`.
- Стабильные CLI-команды: `doctor/new/init/check/build/run/clean/examples`.
- Для `build/run`: поддержка `--cc <compiler>` + host auto-detect.

## 6. Тестовая система

Основные группы:
- lexer/parser/semantic unit + negative,
- codegen smoke и shape/invariants,
- e2e `Skadi -> C -> compile -> run`,
- отдельные сценарии на multi-file import graph.

Быстрый прогон:
```bash
cargo test -q
cargo clippy --all-targets --all-features -- -D warnings
```

## 7. Известные ограничения

- Backend пока только C-transpile (без собственного native backend).
- Часть расширенных design-фич языка отложена на `v1.x/v2`.
- Некоторые runtime-аспекты `struct`/advanced memory model еще закрываются итеративно.

## 8. Где смотреть актуальный статус

- Язык: `docs/SKADI_LANGUAGE_REFERENCE_RU.md`
- Матрица покрытия: `docs/TEST_COVERAGE_MATRIX.md`
- Блокеры v1: `docs/V1_BLOCKERS_MATRIX_RU.md`
- План: `SKADI_IMPLEMENTATION_PLAN.md`
