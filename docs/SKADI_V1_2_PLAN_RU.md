# Skadi v1.2 Plan (RU)

Дата: 2026-07-12
Статус: активная рабочая линия; Memory и Task/Channel runtime slices исполняемы,
release hardening в работе.

## 1. Идентичность релиза

`v1.2` - это первый системный experimental release поверх стабильного `v1.1` toolchain.

Если `v1.1` сделал Skadi цельным продуктовым прототипом с CLI, TUI, formatter, math core, diagnostics и документацией, то `v1.2` проверяет следующую большую гипотезу языка:

```text
Memory        где живут данные
Task/Channel  как течёт работа и сообщения
```

Главная цель `v1.2` - не объявить все системные слои stable, а довести их до честного, тестируемого и хорошо документированного состояния.

## 2. Stable base

`v1.2` наследует stable base из `v1.1`:

- `skadi-cli` и `skadi-cli tui`;
- `check`, `build`, `run`, `format`, `doctor`, `target list`;
- parser / semantic / C codegen для текущего core surface;
- diagnostics в форме `Lex`, `Parse`, `Semantic`, `Codegen`;
- formatter для текущего поддержанного синтаксиса;
- math core;
- I/O builtins;
- showcase smoke coverage;
- HTML documentation workflow.

Эта база не должна ломаться ради experimental tracks.

## 3. Что уже входит в текущий `v1.2` develop

### Memory MVP

Memory model уже находится дальше, чем просто frontend experiment:

- parser принимает `Memory`, `memory(size)`, `place in`, trailing `on error` и `clear`;
- semantic layer проверяет capability-like правила для `Memory`;
- есть escape checks для region-owned dynamic payload;
- запрещены заведомо опасные формы: `Memory` в struct, `Memory List`, copy/reassign handle, return local-region value;
- C backend поддерживает strict fixed-capacity region runtime;
- memory examples и negative suite покрывают контракт.

Ограничения:

- `allow grow`, `allow drop`, `memory.child`, `memory.static` пока остаются design-level future surface;
- allocator policy ещё не является широкой runtime-платформой;
- взаимодействие с будущими Task/Channel runtime rules требует отдельной стабилизации.

### Task/Channel partial runtime MVP

Task model сейчас является experimental runtime MVP:

- parser/AST принимают `Task`, `Task(T)`, `run`, `wait`, `stop`, `stopping`;
- parser/AST принимают `Channel(T)`, `channel(N)`, `send`, `receive`;
- semantic layer проверяет task handle lifecycle;
- `Task` запрещён как обычный storable/returnable value type;
- `wait` и `stop` разрешены только для task handles;
- `stopping` разрешён только в функции, которая локально запускается через `run`;
- `Channel(T)` принимает только value-safe messages;
- игнорирование результата `run worker()` является hard error;
- lifecycle pass требует `wait` на всех путях до выхода из owning scope;
- C backend исполняет void и result-bearing `run/wait` через Win32/pthread runtime;
- CLI автоматически добавляет POSIX pthread link flags;
- cooperative `stop/stopping` работает через синхронизированный Win32/pthread runtime;
- bounded `Channel(T)` исполняет blocking FIFO `send/receive` через Win32/pthread;
- mutable `List` исключён из value-safe сообщений до move/deep-copy контракта;
- активного `SC-CG-301` gate для текущего Task/Channel MVP больше нет.

Task/Channel runtime slice исполняем end-to-end, но concurrency layer остаётся
experimental до release hardening, stress/sanitizer и широкой CI matrix.

## 4. Release goals

Для `v1.2` важно довести до coherence именно системный слой:

- Memory MVP должен оставаться end-to-end: `Skadi -> C -> native`;
- Task/Channel должен иметь ясный frontend contract и исполняемый backend;
- docs должны явно различать stable, partial и future surface;
- examples/showcases должны показывать не только feature count, но и стиль языка;
- diagnostics не должны деградировать при новых правилах;
- formatter/highlighting должны знать новый синтаксис, даже если backend ещё gated.

## 5. Non-goals

Не входит в обязательный scope `v1.2`:

- полноценный OS-thread scheduler;
- async/await;
- task groups;
- `select`;
- non-blocking channel API;
- shared mutable state model;
- `allow grow` / `allow drop` allocator policy;
- child/static memory allocators as stable surface;
- Visual Core runtime;
- modules/imports.

Эти направления можно держать как drafts и future contracts, но не смешивать со стабильным обещанием `v1.2`.

## 6. Milestones

### Milestone 1: Memory consolidation - выполнен

Цель: удержать Memory MVP как рабочий end-to-end слой.

Deliverables:

- memory examples build/run через официальный toolchain;
- negative examples покрывают escape/capability/use-after-clear rules;
- docs прямо говорят, какие memory формы supported, а какие draft-only;
- generated C cleanup остаётся корректным для memory runtime.

Acceptance:

- `cargo test` зелёный;
- memory-specific suites зелёные;
- нет противоречия между language reference, syntax status и MVP contract.

Текущее подтверждение:

- positive и negative memory suites находятся в основном `cargo test`;
- memory examples компилируются в native binaries;
- runtime e2e покрывает fixed-capacity regions, overflow, clear и nested regions.

### Milestone 2: Task/Channel frontend consolidation - выполнен

Цель: закрепить concurrency syntax и semantic contract до backend work.

Deliverables:

- parser/semantic/formatter/highlighting покрывают canonical syntax;
- diagnostics для task/channel rule violations имеют стабильные codes;
- backend gate `SC-CG-301` явно документирован и тестируется на frontend-only этапе;
- examples показывают intended style без обещания runtime execution.

Acceptance:

- `tests/task_model_frontend.rs` зелёный;
- общий `cargo test` зелёный;
- docs называют Task/Channel experimental frontend MVP, а не stable runtime feature.

Текущее подтверждение:

- `tests/task_model_frontend.rs` закрепляет parser, semantic, formatter и backend gate;
- syntax highlighting знает canonical Task/Channel surface;
- до появления runtime `SC-CG-301` не позволял принять frontend support за готовую feature.

### Milestone 3: v1.2 product framing - выполнен

Цель: сделать состояние проекта понятным новому участнику.

Deliverables:

- `v1.1 stable base` и `v1.2 experimental tracks` разведены в docs;
- project overview, syntax status, language reference и coverage matrix синхронизированы;
- docs site menu содержит актуальный v1.2 plan;
- README кратко отражает текущую линию разработки.

Acceptance:

- новый читатель понимает, что можно запускать сейчас;
- новый разработчик понимает, что именно является следующей backend задачей;
- future docs не выглядят как обещание текущего релиза.

### Milestone 4: Task runtime MVP - выполнен

Цель: заменить frontend-only gate первым исполняемым concurrency slice.

Deliverables:

- переносимая runtime abstraction для Win32 и pthread targets;
- lowering для `run`, `wait`, `Task(T)`, `stop` и `stopping`;
- thread-local task и active-memory contexts;
- линейный lifecycle task handle с обязательным единственным `wait`;
- стабильные semantic/codegen/runtime diagnostics для запрещённых и аварийных форм;
- generated C shape tests и native compile/run tests на host toolchain.

Acceptance:

- task-only программы больше не останавливаются на `SC-CG-301`;
- `run -> parallel work -> wait` и `run -> stop -> wait` проходят e2e;
- результат `Task(T)` корректно передаётся ожидающей стороне;
- Memory runtime не содержит process-global active-region race;
- общий compiler и CLI suites остаются зелёными.

Выполненный foundation:

- ignored `run`, repeated stop и незакрытый handle отклоняются через `SC-SEM-070`;
- path-sensitive lifecycle проверяет branch/return/loop boundaries;
- danger entry, capability, region-owned и mutable-list task boundaries запрещены;
- generated C active Memory context переведён на thread-local storage;
- Win32/pthread native regression подтверждает изоляцию active regions.

Выполненный runtime slice:

- platform Task ABI использует Win32 и pthread;
- backend генерирует typed context и trampoline для каждого task entry;
- void `run -> wait` работает в native e2e и через официальный `skadi-cli`;
- после Task-only slice `SC-CG-301` был сужен до Channel и удалён для текущего
  surface после Milestone 5.

Выполненный result slice:

- typed context хранит result worker-функции;
- `wait Task(T)` выполняет join, переносит result и затем освобождает context;
- scalar, struct и Text results проходят native e2e;
- официальный CLI smoke включает result-bearing Task.

Выполненный cooperative-stop slice:

- `stop task` публикует запрос через `Interlocked*` на Win32 и mutex на POSIX;
- `stopping` читает флаг текущей задачи через thread-local current-task pointer;
- `stop` не уничтожает поток и не отменяет обязательный `wait`;
- native e2e и официальный CLI smoke закрепляют `run -> stop -> stopping -> wait`.

Следующий slice: release hardening и проверка новых CI gates.

Архитектурный контракт: [Task Runtime MVP Design](task-runtime-mvp-design.md).

### Milestone 5: Channel runtime MVP - functional slice выполнен

Цель: добавить минимальный message-passing backend поверх работающего Task lifecycle.

Deliverables:

- fixed-capacity bounded FIFO для `Channel(T)`;
- blocking `send` и `receive`;
- platform mutex/condition-variable abstraction;
- typed generated wrappers для value-safe payload;
- явные lifecycle rules: channel owner переживает все задачи-пользователи;
- producer/consumer, backpressure и stress e2e tests.

Acceptance:

- Task/Channel showcase собирается и завершается детерминированно;
- FIFO order и blocking backpressure закреплены тестами;
- region-owned и capability values не пересекают task boundary;
- ThreadSanitizer или доступный платформенный эквивалент не находит гонок в runtime tests.

Выполненный runtime slice:

- generic bounded FIFO хранит payload по value-safe representation;
- Win32 использует `CRITICAL_SECTION` и `CONDITION_VARIABLE`;
- POSIX использует `pthread_mutex` и `pthread_cond`;
- typed wrappers поддерживают scalar, Text и struct messages;
- capacity `1` native e2e закрепляет FIFO и реальный backpressure;
- repeated 1000-message producer/consumer e2e проверяет циклический буфер и signaling;
- owner cleanup выполняется после task `wait`, включая function-local return path;
- `SC-RT-311..313` закрепляют allocation/capacity/synchronization failures;
- официальный CLI smoke и `bench_11_task_channel_pipeline.skd` используют Channel.

### Milestone 6: v1.2 release hardening - в работе

Цель: превратить systems slice в честный release candidate.

Deliverables:

- Memory и Task/Channel showcases через `skadi-cli check/build/run`;
- синхронизация language reference, syntax status, coverage matrix и HTML docs;
- Windows MinGW/MSVC и Linux GCC/Clang CI matrix;
- sanitizer regression flow без отключения проверок;
- обновление package/help/version wording и удаление устаревших raw-driver примеров.

Acceptance:

- full test, CLI smoke и strict docs build зелёные;
- docs однозначно разделяют реализованный runtime и deferred concurrency features;
- активного `SC-CG-301` для текущего Task/Channel surface нет.

Выполнено локально и добавлено в CI contract:

- `bench_12_systems_pipeline.skd` совместно исполняет Memory и Task/Channel;
- showcase входит в обязательный native build gate;
- dedicated `concurrency-tsan` требует успешный GCC ThreadSanitizer stress на Linux
  и не допускает silent skip внутри этого job; binary запускается через
  `setarch x86_64 -R`, чтобы GCC TSan не ломался на ASLR memory mapping до `main`;
- `native-compiler-matrix` собирает и запускает один systems project через Linux
  GCC/Clang и Windows MinGW/MSVC;
- локальные formatter, clippy, full tests и strict docs build остаются release gates.
- отдельное RU/EN руководство по многопоточности фиксирует multi-worker patterns,
  повторный lifecycle, backpressure, deadlock-риски и честный ESP32/RTOS roadmap;
- native runtime suite запускает пять concurrent producers и повторно создаёт
  новый Task handle в каждой итерации.

До закрытия milestone новые remote CI jobs должны пройти на GitHub, после чего
нужно устранить найденные платформенные расхождения без отключения проверок.

## 7. Следующий backend choice

Memory уже закреплён как clean end-to-end reference layer. Текущий порядок работ:

1. прогнать новые TSan и compiler-matrix jobs на GitHub;
2. исправить реальные platform-specific regressions, если matrix их обнаружит;
3. синхронизировать финальный release status и только затем возвращаться к Visual Core.

Task и Channel разделены на два backend milestone намеренно: lifecycle, join и
cooperative stop должны быть проверяемы независимо от блокировок и backpressure
каналов.

## 8. Короткая формула

`v1.1` сделал Skadi удобным прототипом языка и инструментария.

`v1.2` должен показать, что Skadi может расти в сторону системного языка без потери своей главной силы: ясной, спокойной и проверяемой модели кода.
