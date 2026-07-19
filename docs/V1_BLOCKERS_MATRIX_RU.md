# Skadi v1 Blockers Matrix

Дата: 2026-05-25  
Назначение: зафиксировать обязательные решения до стабильного `v1`-релиза транспилятора `Skadi -> C`.

Статус документа: исторический close-out. Все P0 ниже закрыты для контракта `v1`;
актуальная работа ведётся в плане `v1.2`.

## P0 (блокирует релиз v1)

1. Контракт ошибок для выхода за границы (`List`/`Text`) — ЗАКРЫТО ДЛЯ V1
- Решение v1: фиксируем fail-soft поведение (`List` индекс -> `0`, `Text` индекс -> `'\0'`).
- `on error` для индексации в `v1` не вводим.
- Примечание roadmap: возможность перехода к `danger`-контракту рассматривается в `v2+`.

2. Финализация контракта `on error` — ЗАКРЫТО ДЛЯ V1
- Работает для `danger fn` и `List.pop()`.
- Зафиксировано:

  - где `on error` разрешен в v1;
  - какие коды ошибок обязательны (`ErrorCode` policy);
  - поведение для встроенных операций (индексация, I/O, fs).
- Подтверждение:

  - таблица "операция -> может вернуть ошибку -> код";
  - реализация и conformance тесты.
  - разрешённые/запрещённые зоны зафиксированы в `ON_ERROR_V1_MATRIX_RU.md`;
  - semantic/conformance tests закрепляют `danger fn`, builtin и `List.pop()` формы.

3. Формальный контракт `Text` (байты vs символы)
- Сейчас: операции byte-oriented UTF-8.
- Нужно зафиксировать это как публичный v1-контракт, чтобы не было ложных ожиданий про Unicode-графемы.
- Критерий готовности:

  - отдельный раздел в языковой документации;
  - тесты на многобайтные кейсы с ожидаемым поведением.
- Статус: закрыто для v1 (см. `TEXT_V1_CONTRACT_RU.md`, тесты `edge_matrix` и `codegen_e2e`).

4. Политика владения памятью для runtime helper-ов C
- Сейчас есть выделения (`slice`, `concat`, `fs.list`, `read`) без полного контракта владения.
- Нужно зафиксировать:

  - кто освобождает память в v1-пайплайне;
  - где допустимы "процесс-живет-недолго" упрощения;
  - минимальные гарантии отсутствия крашей/UB.
- Критерий готовности:

  - документированный ownership contract;
  - sanitizer e2e проходит стабильно.
- Статус: закрыто для v1 (см. `C_RUNTIME_MEMORY_CONTRACT_V1_RU.md` + sanitizer e2e в `tests/codegen_e2e.rs`).

5. Заморозка каноники синтаксиса v1
- Сейчас часть синтаксиса уже стабилизирована, но остаются переходные зоны.
- Нужно зафиксировать окончательно:

  - `new`, `iterate ... as ...` + поддержка `for ... in ...`,
  - каноника типов (`Int/Float/Bool/Char`, `i32/u32/f64` и т.д.),
  - правила для `return error`, `when`, `my`.
- Критерий готовности:

  - единая таблица "канонично/допускается как алиас/запрещено";
  - style-check и тесты соответствуют таблице.
- Статус: закрыто для v1 (см. `SYNTAX_CANONICAL_MATRIX_V1_RU.md` и style-warning тесты в `semantic_smoke`).

## P1 (желательно до v1.1, но не блокирует v1 core)

1. Согласовать потоковую модель `read/write`
- Сейчас: практичный `read(path)` / `write(path, data)` + `args()`.
- Дальше: спроектировать плавный переход к stream API.

2. Диагностики и коды ошибок
- Статус: закрыто для frontend/toolchain-контракта.
- Стабильные семейства `SC-LEX`, `SC-PARSE`, `SC-SEM`, `SC-MOD` и `SC-CGEN`
  зафиксированы в справочнике кодов и regression-тестах.

3. Расширение e2e-наборов "витринных" программ
- Статус: закрыто и продолжает расширяться вместе с языком.
- Поддерживаемая матрица включает 16 showcase-программ, Memory, Task/Channel,
  Time/Duration, ByteSize и Angle scenarios.

## Close-out P0

- `on error` matrix зафиксирована;
- ownership memory contract для C runtime helper-ов документирован;
- edge/conformance и sanitizer tests входят в текущие gates.


## V1 Reliability Addendum (2026-05-26)

- Added feature-mix codegen e2e scenarios that combine multiple v1 constructs in one program.
- Added golden-lite codegen invariants for critical lowering patterns:

  - `when -> if/else-if`,
  - `danger fn` and `on error` call shape,
  - runtime hooks for `List/Text/fs/io`,
  - statement-only `i++/i--` lowering.
- Added negative compile e2e guard for known semantic/codegen mismatch shape (`output(concat(...))`).
- На момент addendum sanitizer run был optional; текущий `v1.2` CI заменил это
  обязательным dedicated TSan job для Task/Channel runtime.

## V1.1 Scope/Visibility Close-out (2026-07-19)

- запрет shadowing, `local fn/struct/label` и `hide` реализованы и покрыты тестами;
- относительный path-import, direct-import-only видимость и детерминированные коллизии
  закреплены кодами `SC-MOD-001..003`;
- `module.symbol` покрыт для функций, типов структур и вариантов `ErrorCode`;
- канонический типизированный возврат использует `returns`;
- module-name imports и aliases оставлены явным post-v1.1 backlog, а не скрытым блокером.
