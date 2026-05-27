# Skadi Compiler: Project Overview

Дата обновления: 2026-05-27

## 1. Что это за репозиторий

Это рабочий прототип компилятора языка **Skadi** на Rust.
Текущая практическая цель: стабильный и тестируемый pipeline
`lexer -> parser -> semantic -> C transpiler`, плюс удобный CLI-менеджер `skadi-cli`.

## 2. Текущее состояние

Сейчас реализовано:
- лексер (токенизация + диагностика),
- парсер (AST для поддерживаемого подмножества Skadi v1),
- семантический анализ (типы, контексты, ошибки),
- транспиляция в C,
- запуск внешнего C-компилятора через `skadi-cli` (auto-detect/`--cc`),
- мультифайловая сборка через path-import,
- широкий набор unit/integration/e2e тестов.

## 3. Куда смотреть в первую очередь

- Карта документации (какой файл за что отвечает):
  - `docs/DOCS_INDEX.md`
- Язык (текущее реализованное поведение):
  - `docs/SKADI_LANGUAGE_REFERENCE_RU.md`
- Техническая документация по проекту:
  - `docs/SKADI_PROJECT_TECH_REFERENCE_RU.md`
- Статус синтаксиса и v1-ограничения:
  - `docs/SKADI_SYNTAX_STATUS.md`
- Матрица покрытия и блокеры:
  - `docs/TEST_COVERAGE_MATRIX.md`
  - `docs/V1_BLOCKERS_MATRIX_RU.md`
- Релизный freeze-контракт и release notes:
  - `docs/V1_RELEASE_CONTRACT_RU.md`
  - `docs/RELEASE_NOTES_V1_RC1_RU.md`
- План разработки:
  - `SKADI_IMPLEMENTATION_PLAN.md`

## 4. Ключевая структура репозитория

- `src/` — библиотека/ядро компилятора (`lexer`, `parser`, `semantic_analysis`, `codegen`).
- `tests/` — unit/integration/e2e тесты, включая codegen-проверки.
- `tools/skadi-cli/` — менеджер проектов и build/check/run/doctor.
- `docs/` — актуальные спецификации, RFC и матрицы статуса.
- `benchmarks/` — короткие showcase/benchmark-программы на `.skd`.
- `examples/example_tree.skd`, `examples/example_meteostation.txt` — примеры входного кода.

## 5. Быстрый старт для разработчика

Проверка компилятора:
```bash
cargo check
cargo test -q
```

Проверка CLI:
```bash
cargo run -p skadi-cli -- doctor
cargo run -p skadi-cli -- new console demo
cd demo
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- check
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- build
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- run
```

Транспиляция одного файла:
```bash
cargo run -- --input examples/example_tree.skd --print-c
```

## 6. Что важно помнить

- Это **Skadi -> C** транспилятор (не native backend).
- Часть исходного дизайна языка осознанно отложена на `v1.x/v2`.
- Приоритет перед релизом v1: предсказуемость codegen, диагностики и кроссплатформенная стабильность CLI.

