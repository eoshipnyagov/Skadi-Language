# План разработки Skadi

## Дата плана

Исходный план: 2026-05-20

Последняя сверка статусов: 2026-07-30

Документ сохраняет историю этапов. Текущая release-линия и открытые решения
ведутся в [плане v1.2](v1-2-plan.md).

## Цель

Дойти до рабочего MVP-пайплайна компиляции для стабильного подмножества Skadi:

- `lex -> parse -> semantic` для репрезентативных программ;
- детерминированные diagnostics с source locations;
- regression tests для реализованной grammar.

## Этап 0 — Базовая фиксация (завершён)

Статус: completed

Задачи:

- убедиться, что проект собирается через `cargo check`;
- исправить критические compile blockers в lexer/parser contracts;
- заложить project overview и planning docs.

Критерии выхода:

- `cargo check` проходит;
- baseline commit создан.

## Этап 1 — Стабилизация ядра parser (завершён)

Статус: completed

Задачи:

- нормализовать parser entry API в `src/parser/mod.rs`;
- заменить skip-based ветки на явное построение AST для:

  - объявлений функций,
  - присваиваний,
  - `if` / `while` / `loop`,
  - `for in`,
  - каркаса `when` с захватом cases;
- добавить структурированные parse errors (message + token location).

Критерии выхода:

- parser возвращает детерминированные результаты на валидных и невалидных минимальных программах;
- в обычном parse flow нет panic-paths.

## Этап 2 — Движок выражений (Pratt parser, завершён)

Статус: completed

Задачи:

- реализовать precedence table из раннего design-контракта;
- добавить prefix/infix parsing для arithmetic/comparison/logical operators;
- поддержать grouped expressions и variable references.

Критерии выхода:

- корректный по приоритетам AST для выражений;
- тесты хотя бы для 15 сценариев precedence/associativity.

## Этап 3 — Semantic analysis v1 (завершён)

Статус: completed

Задачи:

- scope-aware symbol table validation;
- проверки:

  - use-before-definition,
  - duplicate declarations in same scope,
  - self-reference in first assignment,
  - basic assignment compatibility;
- выдавать user-facing diagnostics с line/column.

Критерии выхода:

- semantic pass ловит core scope/type errors на fixture set;
- формат diagnostics стабилен между запусками.

## Этап 4 — Интеграция и фикстуры

Статус: завершён

Задачи:

- добавить fixture-based tests для:

  - маленьких unit snippets,
  - `examples/example_meteostation.skd` как integration sample;
- добавить pass/fail expectation files;
- добавить CI-friendly test command.

Критерии выхода:

- `cargo test` валидирует MVP grammar slice;
- integration fixture входит в regression suite.

## Этап 5 — Расширение языковых фич (завершён для v1)

Статус: completed

Задачи:

- постепенно реализовывать оставшиеся spec features:

  - `danger fn` + `on error`,
  - `struct`/methods + `my`,
  - выбранные stdlib-aware semantics;
- каждую фичу проводить через tests до merge.

Критерии выхода:

- feature checklist связан с разделами spec и статусом покрытия.

## Этап 6 — Review дизайна языка (завершён для v1)

Статус: completed; future contracts продолжают жить отдельно от stable surface

Задачи:

- пересмотреть `v1` scope языка и явно сократить несущественные фичи для MVP;
- разрешить overlap в syntax/model (`one canonical style per feature in v1`);
- поддерживать принятый bounded Memory contract: segmented `allow grow`,
  declarative `allow drop`, child/root-static regions и явные ограничения;
- держать `docs/SKADI_MEMORY_MODEL_DRAFT_RU.md` как активную reference-точку по memory/lifetime design, пока не зафиксирован более узкий MVP contract;
- держать `docs/SKADI_TASK_MODEL_DRAFT_RU.md` как активную reference-точку по task/channel/concurrency design, пока не зафиксирован более узкий MVP contract;
- держать `docs/SKADI_VISUAL_CORE_DRAFT_RU.md` как широкий future reference, а принятый Canvas v0 фиксировать отдельным MVP contract;
- держать `docs/SKADI_SYSTEMS_ADDITIONS_DRAFT_RU.md` как активную reference-точку по будущему time/units/resource/context/policy design, пока не зафиксирован более узкий MVP contract;
- использовать `docs/SKADI_MEMORY_MODEL_MVP_CONTRACT_RU.md` и `docs/SKADI_TASK_MODEL_MVP_CONTRACT_RU.md` как ближайшие implementation contracts для parser/semantic/runtime planning;
- использовать `docs/SKADI_VISUAL_CORE_MVP_CONTRACT_RU.md` как принятый executable contract для Canvas v0 и основу следующих visual-layer итераций;
- использовать `docs/SKADI_SYSTEMS_ADDITIONS_MVP_CONTRACT_RU.md` как ближайший future implementation contract для time/units/resource/context/policy planning;
- заморозить урезанный `Skadi Core v1` и жёстко привязать compiler milestones к нему;
- синхронизировать syntax decisions с `docs/SKADI_STYLE_PRINCIPLES.md`;
- добавить TODO-трек для human-readable output formatting API, чтобы не уходить в низкоуровневый `%...` formatting noise.

Кандидатное направление:

- читабельный formatter helper для mixed numeric/text output в `v1.x`.

## Отдельный трек — компиляция под target

Статус: partial; desktop native matrix реализована, embedded targets остаются planned

Задачи:

- исследовать и задокументировать cross-target build flows для `Skadi -> C -> target binary`:

  - AVR (embedded),
  - ESP family (Xtensa/RISC-V в зависимости от чипа),
  - ARM targets,
  - Linux targets (`x86_64`/`ARM`, где это практично);
- определить compiler backend/toolchain matrix:

  - required C toolchains per target,
  - minimal build commands,
  - ожидаемые runtime constraints для generated C;
- добавить первый feasibility checklist:

  - "успешно собирает C",
  - "линкует target binary",
  - "гоняет hello-world smoke для target environment/emulator там, где это возможно".

Критерии выхода:

- written design decision record по scope cuts и retained features для `v1`;
- обновлённый grammar/spec section для урезанного core subset;
- implementation plan обновлён так, чтобы приоритизировать только зафиксированные core features.

## Этап 7 — Реализация List/Text v1 (завершён)

Статус: completed

Задачи:

- зафиксировать принятый syntax/typing в RFC (`docs/RFC_LIST.md`, `docs/RFC_TEXT.md`);
- Parser + AST:

  - поддержать `new <Type> List <name> = ...`
  - поддержать list literals `[a, b, c]`
  - поддержать parser shape для indexing в `Text`
- Semantic:

  - enforce list declaration/type rules из RFC
  - валидировать `len(List)` / `len(Text)` и типы индекса
  - валидировать сигнатуры `push`/`pop` и `danger` usage contract
- Codegen/runtime (C):

  - определить `List`/`Text` runtime ABI
  - понизить list/text operations к runtime calls
  - реализовать minimal runtime helpers для операций `v1`
- Diagnostics:

  - добавить стабильные semantic/runtime-facing codes для `List/Text` errors
  - покрыть tests для ожидаемых failure modes

Критерии выхода:

- parser принимает canonical List/Text v1 examples из RFC docs;
- semantic pass валидирует все согласованные правила `v1`;
- C backend может собрать и запустить хотя бы один e2e list/text fixture.

## Этап 8 — Планирование и переход к `v1.1` (завершён)

Статус: completed

Задачи:

- зафиксировать release interpretation `v1` как stable core subset;
- вынести ближайшую post-v1 работу в явные `v1.1` tracks:

  - CLI productization,
  - diagnostics stabilization,
  - docs/showcase synchronization,
  - first math/core slice,
  - I/O/runtime UX alignment;
- держать `v2+` work отдельно от `v1.1`, чтобы не размывать roadmap.

Reference:

- `docs/SKADI_V1_1_PLAN_RU.md`

## Этап 9 — Экспериментальный трек `v1.2`

Статус: release hardening завершён; systems API остаётся experimental

Реализовано и закреплено тестами:

- Memory MVP: region runtime, `place in`, `clear`, базовые escape- и lifecycle-проверки;
- segmented growth, child/root-static regions и bounded ownership transfer
  через `view`/`direct`/`move`;
- Task/Channel runtime MVP: `run`, `wait`, `stop`, `stopping`, bounded FIFO и передача результатов;
- Win32 и pthread backend, многопоточные stress/e2e и обязательный TSan gate в Linux CI;
- относительные path-imports, `local`/`hide`, direct-import-only visibility и `module.symbol`;
- каноническое ключевое слово `returns` для типизированных функций.
- nominal `Time/Duration`, unit literals и monotonic Win32/POSIX runtime;
- `ByteSize`, `Angle` и bounded `Vec2/Vec3/Vec4` end-to-end;
- typed host Interrupt и Canvas v0 с Win32/headless backend;
- переходные формы закрыты compatibility policy или hard diagnostics;
- Windows/Linux/macOS archives, installers, checksums и release workflow;
- `v1.2.0-rc.1` опубликован как draft prerelease.

Открытые задачи и границы текущего этапа ведутся в `docs/SKADI_V1_2_PLAN_RU.md`.
Завершённые runtime slices: cancellation blocking Channel operations,
Duration-bounded `send_for/receive_for` и path-sensitive
`wait task for Duration on error { ... }`. Timed Task wait поглощает handle при
успехе и сохраняет его живым в timeout-handler. Следующий runtime-вопрос — не
новая форма ожидания, а оценка необходимости `select` после укрепления analysis UX.

## Отдельный трек - analysis-first TUI и Skadi-level debugger

Статус: in progress; развивается параллельно runtime backlog и использует тот же
compiler/action pipeline, что CLI и CI.

Архитектурное правило: static analyzer не должен жить внутри rendering/event
loop TUI. Compiler core возвращает структурированные facts, а CLI, TUI,
будущий LSP и CI являются разными клиентами одного analysis engine.

### Фаза A - структурированный анализ

Первые foundation slices выполнены: compiler core возвращает task-aware facts
для blocking/timed Channel, timed Task wait, incomplete `when` и source-order lifecycle facts
для ownership/resources/Task/Memory; CLI action pipeline передаёт их в общий
TUI action history и связывает факты по typed subject. Каждый факт имеет
детерминированный ID и statement-anchor span; `analyze --json` экспортирует
схему `skadi.analysis.v1`. Полные expression spans остаются следующим шагом AST.

- стабильные identifiers и statement-anchor spans — выполнен первый slice;
- facts по creation/copy/borrow/move/close и scope cleanup;
- incomplete `when`, unreachable code и необработанные danger paths;
- потенциально blocking operations и interrupt-context violations;
- Task lifecycle, Channel wait и очевидные циклы ожидания;
- Memory escape, use-after-clear и дорогие implicit copies;
- explain-chain от причины до места пользовательского действия.

### Фаза B - представление в TUI

- отдельные Diagnostics и Lifecycle workspaces с фильтрами
  ownership/resources, Tasks/Channels и Memory — выполнен первый slice;
- source-order история выбранного значения или ресурса — выполнен первый slice;
- места automatic cleanup и раннего `.close()`;
- blocking/capacity state без зависимости только от цвета;
- экспорт тех же результатов в machine-readable CLI output для CI/LSP —
  выполнен первый контракт `skadi.analysis.v1`.

### Фаза C - первый отладчик

- debug build с mapping `Skadi source -> generated C` — выполнен первый
  statement-level slice: `SK-STMT` markers, multi-file origins и sidecar
  `skadi.debug-map.v1`;
- compiler-inserted debug probes — выполнен первый opt-in host slice;
- breakpoints по `.skd`, continue и step — выполнены в CLI;
- отдельный loopback machine channel, call stack, thread ID, базовые
  scalar-locals и Debug workspace в TUI — выполнен первый slice;
- расширенные метаданные Skadi-типов, nested locals и значения ресурсов;
- runtime views для Task, Channel, Memory и linear resources;
- GDB/LLDB могут подключаться как нижний native layer, но TUI не должен
  показывать пользователю generated C как основную модель.

Перед LLVM или вторым backend нужен typed intermediate representation, чтобы
analysis/debug facts не зависели от строкового C-emitter. Собственный machine
debugger, register engine и object-file debugger не входят в цель: Skadi строит
source-level UX поверх существующих native toolchains.

Критерий успеха первой итерации: пользователь может выбрать diagnostic или
resource в TUI и получить короткое точное объяснение его lifecycle и следующего
действия без чтения generated C.

### Фаза D - локальный справочный помощник (future research)

Кандидатное tooling-направление: небольшая локальная нейросетевая модель как
необязательный помощник по справке и объяснению ошибок. Это не часть semantic
core и не источник истины о языке.

Базовая архитектура должна быть `retrieval-first, model-second`:

1. toolchain ищет точные фрагменты в документации текущей версии, diagnostic
   catalog и structured analysis facts;
2. маленькая модель пересказывает найденное простым языком и связывает факты с
   конкретным пользовательским кодом;
3. ответ содержит diagnostic code и ссылки на использованные страницы;
4. при недостатке данных помощник явно сообщает об этом, а не придумывает
   синтаксис или поведение runtime.

Предполагаемые интерфейсы для эксперимента:

- действие `Explain` для diagnostic/lifecycle/debug event в TUI;
- будущая CLI-команда уровня `skadi-cli explain <code>` или объяснение
  последнего результата `check`;
- поиск по справке естественным языком без обязательного доступа к сети.

Ограничения направления:

- обычные diagnostics, docs search, `check` и `analyze` работают без модели;
- локальный/offline режим является предпочтительным и не отправляет исходники
  наружу;
- remote provider может появиться только как отдельная opt-in настройка;
- модель не меняет код автоматически и не повышает warning до error;
- размер, runtime и формат модели выбираются только после измерения качества,
  скорости запуска и потребления памяти на целевых desktop-системах;
- качество проверяется на versioned наборе вопросов и ошибок: отсутствие
  выдуманных форм языка важнее красноречия ответа.

Первый практический spike не требует обучения собственной модели: достаточно
локального индекса документации, стабильного контекста из compiler core и
сменного inference adapter. Собственную специализированную модель имеет смысл
обсуждать только после накопления реальных запросов, где обычного поиска и
детерминированных explain-chain недостаточно.

## Реестр рисков

1. Contract drift между lexer token kinds и parser expectations.  
Митигация: фиксировать shared enums в `common_types.rs` и менять их только вместе с тестами.

2. Рост сложности parser без готового expression engine.  
Митигация: приоритизировать Pratt parser до расширения statement grammar.

3. Регрессии из-за scaffold code paths.  
Митигация: превращать заглушки в явные ошибки там, где поведение ещё не реализовано.

4. Дрейф синтаксиса от целей читаемости.  
Митигация: использовать `docs/SKADI_STYLE_PRINCIPLES.md` как review baseline.

5. Привязка анализа и отладки к C-emitter.
Митигация: вводить typed IR и structured analysis facts до расширения debugger.

## Рабочие правила

- каждая новая grammar feature должна иметь хотя бы один positive и один negative test;
- предпочитать небольшие коммиты по phase tasks;
- использовать `docs/SKADI_SYNTAX_STATUS.md` и пользовательский справочник языка как источник текущего синтаксического контракта;
- синтаксические решения держать синхронизированными с `docs/SKADI_STYLE_PRINCIPLES.md`.
