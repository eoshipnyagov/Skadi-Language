# Skadi Compiler: Project Overview

Дата обновления: 2026-05-26

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

- Язык (текущее реализованное поведение):
  - `docs/SKADI_LANGUAGE_REFERENCE_RU.md`
- Техническая документация по проекту:
  - `docs/SKADI_PROJECT_TECH_REFERENCE_RU.md`
- Статус синтаксиса и v1-ограничения:
  - `docs/SKADI_SYNTAX_STATUS.md`
- Матрица покрытия и блокеры:
  - `docs/TEST_COVERAGE_MATRIX.md`
  - `docs/V1_BLOCKERS_MATRIX_RU.md`
- План разработки:
  - `SKADI_IMPLEMENTATION_PLAN.md`

## 4. Ключевая структура репозитория

- `src/` — библиотека/ядро компилятора (`lexer`, `parser`, `semantic_analysis`, `codegen`).
- `tests/` — unit/integration/e2e тесты, включая codegen-проверки.
- `tools/skadi-cli/` — менеджер проектов и build/check/run/doctor.
- `docs/` — актуальные спецификации, RFC и матрицы статуса.
- `benchmarks/` — короткие showcase/benchmark-программы на `.skd`.
- `example_tree.skd`, `example_meteostation.txt` — примеры входного кода.

## 5. Быстрый старт для разработчика

Проверка компилятора:
```bash
cargo check
cargo test -q
```

Проверка CLI:
```bash
cargo run -p skadi-cli -- doctor
cargo run -p skadi-cli -- new demo
cargo run -p skadi-cli -- check --project demo
cargo run -p skadi-cli -- build --project demo
cargo run -p skadi-cli -- run --project demo
```

Транспиляция одного файла:
```bash
cargo run -- --input example_tree.skd --print-c
```

## 6. Что важно помнить

- Это **Skadi -> C** транспилятор (не native backend).
- Часть исходного дизайна языка осознанно отложена на `v1.x/v2`.
- Приоритет перед релизом v1: предсказуемость codegen, диагностики и кроссплатформенная стабильность CLI.
