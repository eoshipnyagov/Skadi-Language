# Аудит внутренней документации Skadi

Дата сверки: 2026-07-30  
База сверки: ветка `develop`, release candidate `v1.2.0-rc.1`

## 1. Зачем нужен этот документ

Внутренняя документация Skadi содержит четыре разных вида материалов:

1. фактические справочники по текущему компилятору;
2. принятые контракты уже реализованных возможностей;
3. исторические планы завершённых релизов;
4. design drafts и future contracts.

Их нельзя читать как документы одинаковой силы. Этот аудит фиксирует, чему
доверять при разработке, где реализация уже ушла вперёд и какие решения ещё не
приняты.

## 2. Порядок источников истины

При расхождении используйте следующий приоритет:

1. тесты и фактическое поведение `lexer -> parser -> semantic -> codegen -> native`;
2. [статус синтаксиса](../user/syntax-status.md) и
   [быстрая справка](../user/language-quick-reference.md);
3. текущие MVP-контракты;
4. принятые контракты `v1`;
5. release plans и historical close-out;
6. drafts и RFC, которые ещё не получили статус `Accepted`.

Draft описывает направление мысли, но не является обещанием синтаксиса.
Зарезервированный token также не является реализованной конструкцией.

### Синхронизация архитектурного понимания 2026-07-30

Зафиксированы следующие решения:

- Skadi рассматривается как цельная официальная среда, а не только parser
  grammar плюс несвязанные библиотеки;
- официальные слои: Language Core, Systems Core, Domain Core и Platform
  Backends;
- Canvas остаётся Domain Core и учебной/прототипной поверхностью, но не должен
  превращаться в полный GUI/media framework;
- язык должен обучать управлению ресурсами: scope, copy, `view`, `edit`,
  `move`, cleanup, Memory и Task/Channel должны быть видимы в diagnostics;
- `when / is` является строгой формой `switch / case`: single evaluation, no
  fallthrough, duplicate-case diagnostics и будущая exhaustiveness-проверка;
- TUI является first-class интерфейсом к analysis/debug engine, а не отдельной
  реализацией compiler logic;
- первый debugger должен быть Skadi-level: source mapping, probes, breakpoints,
  locals и runtime views поверх C pipeline; собственный machine debugger не
  планируется;
- experimental идеи могут завершаться статусом `Rejected`; совместимость не
  требует сохранять неудачную поверхность до stable/canonical статуса.

## 3. Состояние внутренних документов

| Документ или группа | Роль сейчас | Состояние | Что учитывать |
|---|---|---|---|
| Project Tech Reference | Фактическая архитектура | Актуализирован | Должен меняться вместе с compiler/CLI/runtime |
| Project Overview | Короткий обзор репозитория | Актуализирован | Не заменяет технический справочник |
| CLI Usage | Служебная карта entrypoints | Актуализирован | Пользовательский вход — команда `skadi-cli`, не путь crate |
| CLI RFC v0.1 | История проектирования CLI | Historical / implemented | Не использовать как текущую CLI specification |
| Skadi -> C Scope | Контракт backend | Действующий | Проверять после каждого нового runtime slice |
| Test Coverage Matrix | Карта доказательств | Действующая, ручная | Даты и перечень showcase легко устаревают |
| Token/Construct Matrix | Трассировка форм | Действующая, ручная | Не считать lexer-only token частью языка |
| Diagnostics Style/Codes | Контракт сообщений | Действующий | `SC-CG` и `SC-CGEN` являются разными стадиями |
| Text/List/on error/runtime memory v1 | Замороженная база | Historical accepted | Не расширять задним числом под `v1.2` |
| Scope/Visibility v1.1 | Реализованный контракт | Accepted | Module aliases и re-export в него не входят |
| Plans v1/v1.1 | История релиза | Historical | Не использовать как текущий backlog |
| Plan v1.2 | Текущий release ledger | Living | Реализованные milestones остаются историей выполнения |
| Memory MVP | Experimental runtime contract | Реализованный bounded runtime slice | Есть fixed/grow/child/static regions; полная lifetime theory из Draft не реализована |
| Task/Channel MVP | Experimental runtime contract | Реализованный bounded MVP | Есть close/drain/try_send, timed Channel и timed Task wait; нет async, select и task groups |
| Time/Duration, ByteSize, Angle, Vector | Experimental user/runtime contracts | Реализованы end-to-end | API ещё не объявлен stable |
| Systems Additions MVP | Смешанный implementation/future ledger | Частично реализован | Time/units готовы; resources, context и devices впереди |
| Visual Core Draft/MVP | Draft + accepted Canvas v0 | Реализован experimental Canvas/Win32 slice | Events, text/images, transforms и дополнительные backend впереди |
| Math/Vector RFC | Исторический proposal | Частично принят | Math реализован в `v1.1`, vectors в `v1.2`, Matrix2D отложен |

## 4. Расхождения исходных идей и текущей реализации

### Memory

Исходная идея шире текущего runtime: scope ownership, явная передача владения,
regions и policy-driven bounded allocation.

Сейчас реализованы:

- `Memory name = memory(ByteSize)`;
- segmented growth через `allow grow` без invalidation старых allocations;
- `allow drop` как честный policy marker без automatic reclamation;
- `memory.child(ByteSize)` внутри активного parent-region;
- root-only `memory.static(<ByteSize literal>)`;
- `place in name { ... } on error { ... }`;
- `name.clear()`;
- проверки escape/use-after-clear для динамического payload;
- thread-local active region;
- `move` state machine для `Canvas`, `Window`, `Interrupt` и owning `Channel`;
- explicit move в параметре, call site, binding и factory return.

Пока нет полного lifetime calculus, first-class references, embedded allocator
backend и автоматической стратегии reclamation для `allow drop`. `Memory`
остаётся неперемещаемой region capability, а `Task` использует consume-through-
`wait`.

### Task и Channel

Исходный draft допускает разные backends и дальнейшую structured concurrency.
Текущий MVP использует native Win32/pthread threads, линейные owning handles,
обязательный `wait`, cooperative `stop` и bounded FIFO channels с
`send/receive/try_send/close` и drain-after-close.

Пока нет scheduler abstraction, async/await, task groups, `select`, cancellation
произвольного I/O и RTOS backend. Blocking Channel
`send/receive` отменяются через `stop`; `send_for/receive_for` используют
`Duration` и контекстный `timed_out`. Timed Task wait сохраняет live handle в
timeout-handler и поглощает его только при успешном join.

### Systems types

`Time/Duration`, `ByteSize`, `Angle` и `Vec2/Vec3/Vec4` уже реализованы
end-to-end, хотя ранние drafts описывают их как будущие. Они остаются
experimental не из-за отсутствия runtime, а потому что API ещё не заморожен.

Разница с широким замыслом:

- `Time` только monotonic; wall clock и calendar отсутствуют;
- `Duration` имеет целые `ns/us/ms/s/min/h`, scalar `Int` operations и exact conversion;
- `ByteSize` не является общей dimensional algebra;
- `Angle` хранится в radians; тригонометрия требует nominal `Angle`, inverse
  functions возвращают `Angle`;
- vectors имеют фиксированные `f32` components без generics, SIMD contract,
  swizzling и matrices.

### Numeric ABI и биты

`Int` больше не следует считать alias `i64`: C backend создаёт target-specific
`SkInt`, а `skadi.toml` принимает `[numeric] int = "target|i8|i16|i32|i64"`.
Текущие desktop profiles разрешают `target` в `i32`. Fixed-width типы остаются
независимыми от проекта. Bit builtins работают только с fixed-width signed и
unsigned типами, используют logical right shift и checked indexes. Общая
система checked numeric conversions представлена builtins
`as_i8..as_i64/as_u8..as_u64/as_f32` с обязательным `on error`; float-to-int
принимается только для finite integral значений в диапазоне, а `as_f64` является
safe widening. Width/signedness переменных не смешиваются неявно, литералы
проверяются по диапазону target. Обычная integer arithmetic lowered через
checked helpers и завершает выполнение с `SC-RT-350`; намеренное modulo
поведение вынесено в fixed-width-only `wrapping_add/sub/mul/neg`. Explicit
`f64` сохраняет double-точность литералов и выбирает double math.

### Modules

Реализация использует preprocessing относительных path-imports, local package
imports через `[dependencies]` и правило direct-import-only. Это практический
локальный package layer, но ещё не сетевой package manager.

Не реализованы:

- re-export;
- Git/registry sources, version/transitive resolution и lock-файл;
- отдельная module declaration.

`import "./x.skd" as alias`, `import "package/path/x.skd"` и
квалифицированные `alias.symbol` реализованы.

### Error flow

Текущий контракт намеренно уже универсальных exception/`Result` систем:

- `danger fn`;
- единый `label ErrorCode`, где первый вариант `Ok`;
- `return error Code`;
- обязательный `on error` на danger-вызове.

Индексация остаётся fail-soft и не поддерживает `on error`. Ошибки I/O сейчас
также не создают новый typed error abstraction.

Ранний blockers-документ упоминал stream API как возможное продолжение
`read/write`. Это не принятое обязательство: текущая линия сознательно
стабилизирует небольшой синхронный I/O слой без stream redesign.

### Label и tag

Разрыв закрыт двумя явными nominal-сущностями:

- `label Name { A = 0 B = 1 }` требует числовой дискриминант у каждого варианта;
- `tag Name { A B }` задаёт символические варианты без пользовательских чисел;
- `ErrorCode` остаётся специальным label-контрактом и начинается с `Ok = 0`.

Parser, semantic, formatter, module visibility и C backend поддерживают обе
формы end-to-end.

### Interrupt и embedded

Host MVP поддерживает `Interrupt tick = interrupts.periodic(Duration)` и
`on interrupt tick { ... }` со строгим interrupt-safe подмножеством. Handler
может выполнять конечные scalar-вычисления и `Channel.try_send`, но не blocking,
allocation, I/O или task/resource management.

Desktop target profiles и generated C не означают поддержку ESP32. До
embedded-ready состояния не хватает:

- target/toolchain profile для конкретной платформы;
- runtime adapter для RTOS/bare metal;
- allocation, time, task и I/O contracts без POSIX/Win32;
- linker/flash workflow;
- CI или hardware-in-the-loop gate.

### Visual Core / Canvas

Canvas-first immediate-mode модель перешла в experimental Canvas v0:

- value types `Color`/`Rect`;
- linear resources `Canvas`/`Window`;
- software RGBA framebuffer и deterministic rasterizer;
- line/rect/circle primitives, alpha blending и headless checksum;
- Win32 presenter через `window.present(edit canvas)`.

Пока отсутствуют events, text/images, transforms, `Matrix2D`, non-Windows window
backend и embedded display adapter.

## 5. Зарезервированные и переходные формы

| Форма | Реальный статус | Решение |
|---|---|---|
| `fixed` / `const` | Обычные identifiers | Не резервировать без контракта; неизменяемая форма — только `constant` |
| `edit` / `view` | Реализованный borrow contract | Использовать для явной mutable/read-only передачи без владения |
| `move` | Реализованный bounded ownership transfer | Использовать явно в signature, call site, binding и resource return |
| `allow grow` / `allow drop` | Реализованы только как contextual Memory policies | Не превращать `allow` в общий modifier |
| `on interrupt` | Host MVP для typed periodic Interrupt | Hardware IRQ backend остаётся future |
| `for (init; cond; update)` | Parse/format compatibility + hard rejection | Не использовать в новом коде |
| `fn name(...) Type` | Legacy compatibility | Formatter переводит в `returns Type` |
| `for ... in ...` | Работает | Каноническая витринная форма всё ещё `iterate ... as ...` |
| `bool` / `char` | Работают как aliases | В новом коде писать `Bool` / `Char` |

## 6. Что ещё предстоит сделать

### После checkpoint 2026-07-30

- пользовательская и внутренняя RU/EN документация, syntax status, coverage
  matrices и HTML-навигация синхронизированы;
- Memory/ownership/Canvas surfaces остаются experimental и не требуют знания
  расширенных policies в quick-start workflow;
- перед финальным следующим release нужен повторный прогон на чистых
  Windows/Linux/macOS окружениях;
- прежняя фиктивная резервация `fixed`/`const` удалена из документации и
  подсветки; оба слова являются обычными identifiers.

### Следующая функциональная очередь

1. Пробуждение blocking Channel operations при `stop`/`close` и единая
   cancellation semantics на Win32/pthread — выполнено.
2. Timed Channel operations и path-sensitive timed Task wait на основе
   `Duration` — выполнены. `select` обсуждать только после укрепления анализа
   blocking/lifecycle цепочек.
3. Подключать Resource lifecycle для файлов, портов и device handles только
   вместе с появлением соответствующих долгоживущих API.
4. Hardware interrupt binding поверх готового host periodic/semantic MVP.
5. Embedded target contract и первый ESP32/FreeRTOS spike.
6. `Ring`/bounded `Pool` для явной `drop oldest` семантики.
7. Canvas events, text/images и следующие presentation backends.
8. Ограниченный C ABI slice включает `external fn`, fixed scalar types,
   call-scoped `view`/`edit Buffer(T)` и `[native]` sources/libraries. Первый
   local-path package resolver через `[dependencies]` также готов и сохраняет
   direct-import-only/diamond-dedup контракты. Следующая очередь: Git/registry
   resolver, transitive graph и lockfile, custom/packed struct layout, opaque
   handles и видимое владение долгоживущими C resources; by-value
   `external struct` с fixed scalar fields уже реализован. Следующие формы проверяются на
   реальных C-библиотеках до заморозки.

### Параллельный tooling-трек

Tooling развивается параллельно runtime-очереди, не копируя semantic logic в
TUI:

1. structured analysis foundation для incomplete `when` и blocking Channel —
   выполнен первый slice;
2. source-order explain-chain для ownership/resource lifecycle и
   Task/Channel/Memory state — выполнен первый slice;
3. отдельный Lifecycle workspace с фильтрами ресурсов, задач, каналов и
   регионов — выполнен первый slice;
4. statement-level source mapping с multi-file origins и build sidecar —
   выполнен первый slice; debug probes, CLI breakpoint/step, loopback machine
   channel, call stack, scalar-locals и TUI Debug workspace готовы; nested
   locals и runtime views ресурсов впереди;
5. общий engine для CLI, TUI, будущего LSP и CI — первый JSON-контракт
   `skadi.analysis.v1` доступен через `skadi-cli analyze --json`.
6. Локальный справочный AI-помощник — future research: retrieval по versioned
   docs и structured facts остаётся источником истины, а маленькая модель может
   только объяснять найденное; offline-first, без обязательной сети и без
   автоматического изменения кода.

### Осознанно не надо добавлять без отдельного design decision

- generics ради самих generics;
- decorators/annotations как универсальную метапрограмму;
- operator overloading без жёсткой системной необходимости;
- implicit async runtime;
- скрытый shared mutable state;
- большую dimensional-units систему до практического embedded/use-case.
- полный GUI/media/game framework внутри Canvas Core;
- собственный machine-code debugger вместо source-level интеграции с готовыми
  native toolchains.

## 7. Правило дальнейшей синхронизации

Любая новая языковая форма должна одновременно обновлять:

- parser/semantic/codegen/runtime;
- formatter;
- VS Code и Pygments syntax highlighting;
- quick reference и соответствующую подробную страницу;
- syntax status;
- positive/negative/e2e tests;
- token/construct и test coverage matrices.
- structured analysis/TUI/LSP consumers, если новая форма создаёт lifecycle,
  blocking или ownership facts.

Если слой отсутствует, форма получает явный статус `Compatibility`,
`Reserved` или `Future`, а не расплывчатое «поддерживается частично».
