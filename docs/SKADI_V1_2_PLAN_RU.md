# Skadi v1.2 Plan (RU)

Дата: 2026-07-19
Статус: release hardening завершён; Memory, Task/Channel, Time/Duration,
ByteSize и Angle slices исполняемы и остаются experimental API текущей линии
`v1.2`.

## 1. Идентичность релиза

`v1.2` - это первый системный experimental release поверх стабильного `v1.1` toolchain.

Если `v1.1` сделал Skadi цельным продуктовым прототипом с CLI, TUI, formatter, math core, diagnostics и документацией, то `v1.2` проверяет следующую большую гипотезу языка:

```text
Memory        где живут данные
Task/Channel  как течёт работа и сообщения
Time/Duration как выражаются интервалы и monotonic-время
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

Task/Channel runtime slice исполняем end-to-end и проходит stress/sanitizer и
широкую CI matrix. Concurrency layer остаётся experimental из-за незамороженного
API, а не из-за отсутствующего backend.

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
- module-name imports, aliases и re-exports поверх реализованных path-imports.

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

Следующим slice стал Channel runtime MVP из Milestone 5; release hardening закрыт
в Milestone 6.

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

### Milestone 6: v1.2 release hardening - выполнен

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

Выполнено локально и подтверждено remote CI:

- `bench_12_systems_pipeline.skd` совместно исполняет Memory и Task/Channel;
- showcase входит в обязательный native build gate;
- dedicated `concurrency-tsan` требует успешный GCC ThreadSanitizer stress на Linux
  и не допускает silent skip внутри этого job; binary запускается через
  `setarch x86_64 -R`, чтобы GCC TSan не ломался на ASLR memory mapping до `main`;
- `native-compiler-matrix` собирает и запускает один systems project через Linux
  GCC/Clang и Windows MinGW/MSVC;
- formatter, clippy, full tests и strict docs build остаются release gates;
- отдельное RU/EN руководство по многопоточности фиксирует multi-worker patterns,
  повторный lifecycle, backpressure, deadlock-риски и честный ESP32/RTOS roadmap;
- native runtime suite запускает пять concurrent producers и повторно создаёт
  новый Task handle в каждой итерации.

Remote GitHub Actions matrix зелёная на Windows MinGW/MSVC, Linux GCC/Clang и
macOS. Обнаруженные platform-specific расхождения исправлены в implementation,
без отключения sanitizer или ослабления test gates.

### Milestone 7: Time/Duration runtime MVP - functional slice выполнен

Цель: проверить фундамент nominal specialized types на маленьком системном API.

Реализовано:

- nominal `Time` и `Duration` без неявного смешивания с `Int/Float`;
- overflow-checked integer literals `ms`, `s`, `min`;
- явная арифметика `Time/Duration` и сравнения одинаковых типов;
- `now`, `elapsed`, `sleep`, `delay`;
- signed `i64` nanoseconds и monotonic Win32/POSIX C runtime;
- value-safe integration со struct, List, `Task(T)` и `Channel(T)`;
- parser/semantic/formatter/codegen/native e2e coverage;
- `bench_13_time_budget.skd` в showcase build/smoke flow;
- отдельное RU/EN пользовательское руководство.

Границы:

- API остаётся experimental;
- `Time` не является wall-clock или Unix timestamp;
- fractional literals, `Timer`, timed channel/task operations и embedded backend
  не входят в этот slice.

## 7. Порядок завершения текущей линии

Remote compiler matrix для `Time/Duration` зелёная. Дальнейшая работа в текущей
линии идёт в следующем порядке и не расширяется release-packaging задачами.

### Milestone 8: документационная сверка

Цель: ещё раз сопоставить пользовательские и внутренние документы с фактическими
parser, semantic, formatter, codegen, CLI и TUI contracts.

Обязательный результат:

- устранены устаревшие пути, версии, статусы и противоречивые release-формулировки;
- language reference и syntax status покрывают все реализованные конструкции;
- примеры кода проходят текущий formatter/check flow;
- RU/EN HTML-навигация и generated site собираются в strict mode;
- historical/future документы не выглядят как обещание текущего runtime.

### Milestone 9: ByteSize MVP - functional slice выполнен

Цель: выделить размеры памяти из контекстных числовых литералов в отдельный
nominal type и связать его с существующим Memory MVP.

Реализовано:

- `ByteSize` и integer literals `b`, `kb`, `mb`, `gb`;
- overflow-checked parser representation;
- запрет неявного смешивания с `Int/Float`;
- явно зафиксированная арифметика и сравнения;
- `memory(ByteSize)` без расширения allocator policy;
- formatter, highlighting, diagnostics, codegen и native e2e;
- отдельный небольшой showcase и RU/EN пользовательская документация.

Контракт зафиксирован: signed `i64` bytes, бинарные множители, отрицательные
промежуточные значения и runtime rejection для non-positive Memory capacity.
`allow grow`, `allow drop`, `memory.child` и `memory.static` не входят в этот
milestone.

### Milestone 10: Angle MVP - functional slice выполнен

Цель: убрать неоднозначность между градусами, радианами и обычными `Float`, не
создавая общего framework физической размерности.

Реализовано:

- nominal `Angle` и literals `deg`, `rad`;
- overflow/finite-value policy и явно выбранное внутреннее представление;
- сравнения, сложение/вычитание углов и ограниченные scalar operations;
- явные conversions между градусами и радианами;
- интеграция с `sin`, `cos`, `atan2` без параллельного двусмысленного API;
- formatter, highlighting, diagnostics, C lowering, e2e и небольшой showcase;
- RU/EN пользовательская документация.

Контракт зафиксирован: `f64` radians, finite integer/fractional literals,
nominal arithmetic и временная numeric-radians compatibility только для
`sin/cos`. Общая dimensional algebra, angular velocity и implicit
`Float <-> Angle` conversions не входят в MVP.

### Milestone 11: Vector MVP - functional slice выполнен

Цель: добавить первый ограниченный math value layer после серии проверенных
nominal-type slices.

Реализовано:

- `Vec2`, `Vec3`, `Vec4` с одним явно выбранным scalar representation;
- создание, доступ к компонентам и value semantics;
- векторные `+`/`-` и умножение/деление на scalar;
- `dot`, `length`, `length_sq`, `normalize`, `distance`, `distance_sq`;
- `cross` только для `Vec3`;
- semantic negative coverage, C lowering, e2e и showcase.

Контракт зафиксирован: `f64` components, точные structural literals,
same-dimension operations, zero-safe normalization и value-safe boundaries.

Матрицы, SIMD-specific lowering, generic vectors, swizzling и пользовательский
operator overloading не входят в первый vector slice. Реализованный перед ним
`Angle` может использоваться в связанных helper APIs, но не расширяет сам
vector slice до матриц или transform framework.

### Milestone 12: переходные поверхности - frontend pass выполнен

После завершения ByteSize, Angle и Vector MVP закрыты формы, которые выглядели
частично реализованными:

- `on interrupt` остаётся parse/format future form, но получает обязательный `SC-SEM-040`;
- compound assignments десахарируются без потери левого операнда;
- formatter канонизирует typed returns через `returns` и compound assignments через явную форму;
- legacy typed-return syntax принимается с warning для compatibility;
- legacy C-style `for` остаётся parse/format compatibility-формой и получает обязательный `SC-SEM-040` вместо тихой потери в codegen;
- `Char` literals получили ASCII/escape contract и `SC-PARSE-221`;
- scalar `new name = expr` получает корректный inferred C type, а composite declaration без явного типа останавливается через `SC-SEM-020`;
- остаточные `Partial/Planned` пометки сверены с diagnostics и tests;
- не превращать module aliases, indexing `on error` и расширенный concurrency API
  в неявные обязательства этой линии.

### Следующий спринт: v1.2 Distribution & Release Candidate

Цель: превратить завершённую функциональную линию в воспроизводимый
дистрибутив, который устанавливается без клонирования репозитория и без Rust.
Первая контрольная версия — `v1.2.0-rc.1`; после проверки тех же артефактов
выпускается `v1.2.0`.

Обязательный scope:

- синхронизировать package/version identity в Cargo metadata, `skadi-cli`,
  TUI, документации и generated site;
- полностью обновить корневой `README.md` под фактический `v1.2`: статус,
  реализованные Memory/Task/Channel/Time/ByteSize/Angle/Vector slices,
  installation quick start, CLI/TUI workflow, ссылки на HTML-документацию и
  честные experimental/deferred границы;
- сохранить `skadi-cli` как установленное имя команды на этом release cycle;
- добавить `CHANGELOG.md` и краткую миграцию `v1.1 -> v1.2`;
- собирать versioned release archives для Windows, Linux и macOS;
- публиковать SHA-256 checksum manifest рядом с артефактами;
- добавить user-local installer и uninstaller для PowerShell и POSIX shell;
- добавить tag-driven GitHub Actions workflow, который сначала создаёт draft
  Release и не обходит обычные quality gates;
- запускать distribution smoke tests из чистого временного HOME/PATH через
  установленный бинарник: `new`, `check`, `format`, `build`, `run`, `doctor`,
  `target list`;
- документировать ручную/offline установку из скачанного архива.

#### Контракт release-артефактов

Release workflow определяет платформу по Rust target и публикует однозначно
названные архивы:

- Windows x86-64: MSVC binary в `.zip`;
- Linux x86-64 и ARM64: по возможности статические `musl` binaries в
  `.tar.gz`, чтобы CLI не зависел от версии glibc конкретного дистрибутива;
- macOS Intel и Apple Silicon: отдельные binaries или один проверенный universal
  binary в `.tar.gz`.

Каждый архив содержит `skadi-cli`, license/readme fragment и version metadata.
Checksum проверяется installer-ом до распаковки. Замена существующей версии
выполняется через временный файл и atomic rename, чтобы не оставлять частично
установленный бинарник.

#### Windows installer

`install.ps1`:

- определяет архитектуру и запрошенную версию;
- скачивает release archive и checksum manifest;
- по умолчанию устанавливает в
  `%LOCALAPPDATA%\Programs\Skadi\bin\skadi-cli.exe`;
- идемпотентно добавляет этот каталог в пользовательский `PATH`, не требуя
  Administrator;
- поддерживает явные `-Version`, `-InstallDir` и повторный запуск для обновления;
- после установки запускает `skadi-cli --version` и `skadi-cli doctor`.

`uninstall.ps1` удаляет только файлы, записанные installation manifest, и только
добавленную installer-ом запись `PATH`. Пользовательские проекты и конфигурация
не удаляются.

#### Linux installer

`install.sh` определяет `uname -s`/`uname -m`, выбирает `musl` artifact и по
умолчанию устанавливает `skadi-cli` в `~/.local/bin`. Это подходит для обычных
glibc-дистрибутивов, Alpine, WSL и других Linux-систем на поддержанной
архитектуре.

Если `~/.local/bin` отсутствует в `PATH`, installer либо добавляет отдельный
маркированный блок в профиль поддержанного shell, либо при `--no-modify-path`
печатает точную команду для ручной настройки. `--system` использует
`/usr/local/bin` и запрашивает `sudo` только для операции копирования.
`uninstall.sh` использует installation manifest и снимает только собственные
изменения.

#### macOS installer

Тот же POSIX installer выбирает Intel/Apple Silicon artifact и устанавливает его
в `~/.local/bin` или `/usr/local/bin` при `--system`. На RC binaries могут быть
без Apple notarization: installer не должен молча отключать Gatekeeper или
удалять quarantine attributes. Ограничение и ручной путь запуска документируются
явно; signing/notarization включается только при наличии доверенных release
credentials.

#### Внешний C toolchain

Installer устанавливает сам Skadi toolchain, но не устанавливает системный C
compiler без согласия пользователя. `check`, `format` и TUI не требуют C
compiler; `build` и `run` требуют host toolchain. Финальный `doctor` должен
давать platform-specific подсказки:

- Windows: MSVC Build Tools или MinGW;
- macOS: Xcode Command Line Tools;
- Linux: GCC/Clang через обнаруженный package manager (`apt`, `dnf`, `pacman`,
  `zypper`, `apk`).

Release acceptance:

- installer работает без Rust и checkout репозитория;
- checksum mismatch всегда останавливает установку;
- повторная установка обновляет бинарник идемпотентно;
- uninstall не удаляет пользовательские данные;
- установленный `skadi-cli` проходит полный smoke flow на Windows, Linux и
  macOS;
- корневой README и HTML-документация описывают именно опубликованную версию.

`Timer`, wall-clock/calendar API, общая dimensional algebra, operator overloading,
TOML editor и новые TUI-функции не входят в distribution sprint.

Полный future contract: [Systems Additions MVP](systems-additions-mvp.md).

## 8. Короткая формула

`v1.1` сделал Skadi удобным прототипом языка и инструментария.

`v1.2` должен показать, что Skadi может расти в сторону системного языка без потери своей главной силы: ясной, спокойной и проверяемой модели кода.
