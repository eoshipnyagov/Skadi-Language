# Skadi v1 Release Contract (Draft for Approval)

Date: 2026-05-27
Status: draft (to be approved)

## 1. Цель v1

Стабильный и предсказуемый pipeline:
`Skadi source -> lex -> parse -> semantic -> C -> native compile/run`,
с детерминированной диагностикой и кроссплатформенным CLI-потоком.

## 2. Входит в v1 (release scope)

1. Компиляторный pipeline:
- lexer/parsing/semantic/codegen для текущего зафиксированного синтаксического среза.

2. CLI-менеджер:
- `doctor`, `new`, `init`, `examples`, `check`, `build`, `run`, `clean`, `target list`, `tui`.

3. Модульная сборка:
- только `import "./relative_path.skd"` (recursive, dedup, cycle-safe, deterministic order).

4. Диагностики:
- стабильные коды и stage ownership:
  - parser: `SC-PARSE-*`
  - semantic: `SC-SEM-*`
  - module/import: `SC-MOD-001`
  - native compile: `SC-CGEN-001`
  - pipeline wrappers: `SC-LEX-000`, `SC-PARSE-000`, `SC-SEM-000`
- формат pipeline сообщений: `code + stage + hint`.

5. Тестовый барьер:
- обязательные unit/integration/e2e тесты,
- усиленный `codegen_e2e` набор (feature-mix + stress + negative),
- import-graph e2e/negative в `tools/skadi-cli`.

## 3. Не входит в v1 (deferred)

1. Расширение модульной системы:
- `import module_name`, alias (`as`), visibility rules.

2. Расширенный runtime/concurrency:
- `run/wait/Link`, runtime semantics для `on interrupt/on event`.

3. Расширение языка за пределами текущего freeze:
- `direct`, `returns struct`, `local fn`, chunk-memory (`allow drop`, budgets) как production-contract.

4. Math/vector core как обязательная часть релиза:
- переносится в `v1.x`.

## 4. Критерии готовности v1

1. Локально:
- `cargo test -q` green
- `cargo clippy` green (root + `tools/skadi-cli`)
- CLI smoke: `doctor`, `new`, `check`, `build`, `run`

2. На GitHub:
- CI matrix (Win/macOS/Linux) green:
  - `test-matrix`
  - `codegen-e2e`
- `sanitizer-optional` дает pass или explicit skip с логом.

## 5. Пункты на согласование (требуют подтверждения)

1. Freeze для модулей:
- подтверждаем, что для v1 каноника только `import "./... .skd"`.

2. Freeze для диагностик:
- подтверждаем, что wrapper-коды `SC-LEX-000/SC-PARSE-000/SC-SEM-000` являются публичным контрактом CLI.

3. Freeze для scope:
- все крупные расширения синтаксиса/рантайма уходят в `v1.x/v2`, без расширения surface в `v1.0.0`.
