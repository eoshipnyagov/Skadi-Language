# Skadi v1 Blockers Matrix

Дата: 2026-05-27
Назначение: фиксировать обязательные решения перед стабильным `v1` релизом транспилятора `Skadi -> C`.

## P0 (блокирует v1)

1. Стабильность codegen e2e (feature-mix программы) — ЗАКРЫТО
- Что уже закрыто:
  - расширенная матрица `tests/codegen_e2e.rs` (26 сценариев),
  - добавлены mutation-like негативные e2e в `tools/skadi-cli/src/pipeline.rs`.
- Что осталось:
  - поддерживать матрицу синхронно с новыми фичами (без снижения покрытия).

2. Единообразие диагностик parser/semantic/codegen — ЗАКРЫТО
- Что уже закрыто:
  - унифицирована ошибка native C compile с кодом `[SC-CGEN-001]` и матрицей попыток компиляторов,
  - импортный контракт стабильно маркируется как `[SC-MOD-001]`.
  - в `compile_to_c` добавлены stage-wrapper коды (`SC-LEX-000`, `SC-PARSE-000`, `SC-SEM-000`),
  - формализован единый префикс `code + stage + hint`,
  - reference-файл с ownership и кодами: `docs/DIAGNOSTIC_CODES_REFERENCE.md`,
  - добавлены контрактные mutation-тесты на стабильный формат диагностик по стадиям.

3. Multi-file import contract (`import "./... .skd"`) + edge-cases — ЗАКРЫТО

4. Кроссплатформенный CLI pipeline (Win/Linux/macOS) + doctor — В РАБОТЕ
- Что уже закрыто:
  - добавлен GitHub Actions workflow с матрицей `ubuntu/windows/macos`,
  - выделен отдельный required job `codegen-e2e` для compile/run защиты,
  - `test-matrix` оставлен для non-e2e + `tools/skadi-cli` tests + smoke compile + `doctor`,
  - добавлен `sanitizer-optional` (ASan/UBSan) с явным логированием.
- Что осталось:
  - пройти реальный CI green-run на GitHub и зафиксировать baseline.

## P1 (можно в v1.x)

1. Расширение `on error` beyond danger/list-pop
2. Расширение struct/method lowering
3. Math/vector core API
4. Offline docs UX (`skadi docs`) и LLM-guide генерация

## Техдолг

- Финальная полировка всех RU/EN доков и перекрестных ссылок
- Дополнительные negative тесты на редкие edge-cases parser/semantic
- Дополнительные invariant-проверки generated C
- Явная матрица покрытия по лексемам/конструкциям:
  - `keyword/lexeme -> lexer/parser/semantic/codegen/e2e`,
  - отметить все пробелы как pre-freeze TODO.

## Принцип закрытия пункта

Пункт считается закрытым только если есть:
- код,
- тесты,
- синхронное обновление документации.
