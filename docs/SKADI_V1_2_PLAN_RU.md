# Skadi v1.2 Plan (RU)

Дата: 2026-06-21  
Статус: активная рабочая линия после `v1.1`.

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

### Task/Channel frontend MVP

Task model сейчас является experimental frontend surface:

- parser/AST принимают `Task`, `Task(T)`, `run`, `wait`, `stop`, `stopping`;
- parser/AST принимают `Channel(T)`, `channel(N)`, `send`, `receive`;
- semantic layer проверяет task handle lifecycle;
- `Task` запрещён как обычный storable/returnable value type;
- `wait` и `stop` разрешены только для task handles;
- `stopping` разрешён только в функции, которая локально запускается через `run`;
- `Channel(T)` принимает только value-safe messages;
- игнорирование результата `run worker()` остаётся warning;
- C backend намеренно останавливается на `SC-CG-301`, потому что runtime/backend concurrency ещё не реализованы.

Этот слой уже полезен как контракт языка и как regression surface, но пока не является runtime feature.

## 4. Release goals

Для `v1.2` важно довести до coherence именно системный слой:

- Memory MVP должен оставаться end-to-end: `Skadi -> C -> native`;
- Task/Channel должен иметь ясный frontend contract и подготовленный backend plan;
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

### Milestone 1: Memory consolidation

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

### Milestone 2: Task/Channel frontend consolidation

Цель: закрепить concurrency syntax и semantic contract до backend work.

Deliverables:

- parser/semantic/formatter/highlighting покрывают canonical syntax;
- diagnostics для task/channel rule violations имеют стабильные codes;
- backend gate `SC-CG-301` явно документирован и тестируется;
- examples показывают intended style без обещания runtime execution.

Acceptance:

- `tests/task_model_frontend.rs` зелёный;
- общий `cargo test` зелёный;
- docs называют Task/Channel experimental frontend MVP, а не stable runtime feature.

### Milestone 3: v1.2 product framing

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

## 7. Следующий backend choice

После текущего frontend pass есть два разумных пути:

- стабилизировать Memory дальше как первый настоящий systems layer;
- начать Task/Channel backend MVP.

Рекомендуемый порядок:

1. сначала удержать Memory как clean end-to-end reference layer;
2. затем проектировать минимальный Task/Channel backend поверх уже понятных resource/value rules;
3. только после этого возвращаться к Visual Core и systems additions.

Причина простая: Task/Channel без ясной Memory/value-safe границы быстро превращается в runtime-комбайн. Skadi лучше двигать маленькими, проверяемыми слоями.

## 8. Короткая формула

`v1.1` сделал Skadi удобным прототипом языка и инструментария.

`v1.2` должен показать, что Skadi может расти в сторону системного языка без потери своей главной силы: ясной, спокойной и проверяемой модели кода.
