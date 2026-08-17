# Матрица тестового покрытия (stable `v1.1` + experimental `v1.2`)

Дата: 2026-07-29
Ответственный слой: Skadi core

Этот файл фиксирует покрытие тестами для элементов языка в текущем Rust-прототипе.

## 1. Что уже покрыто

- лексинг, токенизация и diagnostics
  - `tests/lexer_smoke.rs`
- core parser для statements и expressions
  - `tests/parser_smoke.rs`
  - включает: `new`, typed `new`, `if/else`, `while`, `for in`, `iterate ... as ...`,
    объявления функций, `danger fn`, `return`, `return error`, `when/is/else`, `label`,
    форму `struct`, `local fn/struct/label`, `hide`, канонический `returns`, qualified names,
    `on interrupt`, list literals, `push/pop-on-error`, indexing, calls и `break/continue/pass`
- semantic validation
  - `tests/semantic_smoke.rs`
  - включает: type mismatch, scope/redeclaration, use-before-def, arity/types calls,
    проверки связки `danger` + `on error`, правила `ErrorCode`, типизацию list/text,
    вывод типов для `for/iterate`, проверки text builtins (`len/contains/find/slice`),
    негативные проверки `on error` на не-danger builtins (`read/write/fs.list`),
    стилевые предупреждения (`iterate`, `Bool/Char`)
  - включает loop-context rules для `break/continue`
  - включает scope/visibility rules: запрет shadowing, локальные объявления, скрытые поля,
    qualified function/struct/ErrorCode references
- shape-проверки codegen
  - `tests/codegen_smoke.rs`
  - включает понижение control flow, `when`, runtime-вызовы для list/text,
    shape danger-call lowering, typed declarations
- интеграционные pipeline-тесты
  - `tests/language_programs.rs`
  - end-to-end через `lex -> parse -> semantic -> C generation` для многофичевых программ
  - включает compile-checked `examples/language/01_small_features.skd` для
    word operators, fixed-width values, List, struct/method и danger flow
- multi-file/import pipeline
  - `tools/skadi-cli/src/pipeline.rs` проверяет относительные path-imports, отсутствующие файлы и циклы
  - закреплены `SC-MOD-001`, `SC-MOD-002` и `SC-MOD-003`
  - покрыты direct-import-only visibility, public-symbol collisions, `local` isolation и
    `module.symbol` для функций, структур и вариантов `ErrorCode`
- e2e-тесты с C-компилятором
  - `tests/codegen_e2e.rs`
  - C output собирается, а binaries запускаются на representative programs
  - включает edge-сценарии для `Text`, пустой needle, `List` + `when`
  - включает UTF-8 smoke-сценарий с byte-semantics
  - включает compile/run-сценарий для math core
  - включает sanitizer-backed stress scenario (`ASan/UBSan`), когда toolchain поддерживает флаги
  - включает memory contract tie-in: отсутствие sanitizer-detected crashes/UB в текущей runtime allocation model
  - включает feature-mix scenarios (`struct+method`, `iterate/when`, `i++/i--`, `io/fs`)
  - включает negative compile e2e guard для известной semantic/codegen mismatch shape
- edge matrix conformance set
  - `tests/edge_matrix.rs`
  - включает:

    - numeric `List` coverage для семейств `i/u/f` (`8/16/32/64`) и `bool`
    - понижение `Path List` к text runtime helpers
    - extreme text index/slice shapes
    - UTF-8 text contract shape (byte-oriented `len/index/slice`)
    - negative builtin argument/type checks (`fs.join`, `write`, `args`)
    - обход списка структур и method calls
    - `danger` + `on error` + explicit `ErrorCode`
- math core coverage
  - semantic positive/negative checks для constants и numeric builtin typing
  - codegen shape checks для `math.h`, constants, trigonometry, `root`, angle conversion
  - showcase coverage через `bench_09_math_navigation.skd` и `bench_10_v1_1_toolbox.skd`
- showcase coverage
  - compile-pipeline shape tests покрывают `bench_01..16`
  - native build suite подтверждает `Skadi -> C -> native exe` для `bench_01..16`
  - runtime showcase e2e покрывает:
    - CLI-driven subset `bench_01..05`
    - stable subset `bench_06..09`
    - dedicated full showcase `bench_10_v1_1_toolbox.skd`
    - concurrency showcase `bench_11_task_channel_pipeline.skd`
    - combined systems showcase `bench_12_systems_pipeline.skd`
    - time budget showcase `bench_13_time_budget.skd`
    - memory budget showcase `bench_14_byte_size_budget.skd`
    - angle navigation showcase `bench_15_angle_navigation.skd`
    - vector navigation showcase `bench_16_vector_navigation.skd`
    - headless Canvas palette showcase `bench_17_canvas_palette.skd`
  - showcase fixtures лежат в `benchmarks/showcase-data/` и используются в script/e2e smoke-path
- experimental memory frontend coverage
  - `tests/memory_model_frontend.rs` проверяет parser/formatter/semantic/codegen
    contract для fixed/growing/child/root-static `Memory`, `place in`, `clear`,
    policy errors, escape rules и illegal `Memory` usage
  - `tests/memory_model_examples.rs` проверяет self-contained positive examples,
    segmented growth, child/static native runtime, большой canonical example,
    TLS isolation, style pitfalls и negative suite из `examples/memory/`
- experimental ownership-transfer coverage
  - `tests/canvas_model.rs` проверяет `move` parameter/call/return, use-after-
    move, partial branch move, loop rejection, factory native execution и
    `examples/ownership/01_move_canvas_factory.skd`
  - `tests/systems_contract_frontend.rs` закрепляет Channel/Window/Interrupt
    transfer и соответствующие C move/cleanup helpers
- experimental task/channel frontend coverage
  - `tests/task_model_frontend.rs` проверяет parser/semantic contract для `Task`,
    `run`, обычного/timed `wait`, `stop`, `stopping`, `Channel(T)`, `channel(N)`,
    blocking/timed `send` и `receive`
  - тот же suite проверяет ignored-run hard error, all-path lifecycle,
    task-safe boundaries и value-safe channel messages, включая запрет mutable `List`
  - memory suites проверяют TLS shape и native Win32/pthread isolation active regions
  - `tests/task_model_runtime.rs` проверяет native void и result-bearing tasks,
    typed arguments, scalar/struct/Text result transfer, обычный и timed join,
    `stop -> stopping -> wait`,
    bounded FIFO, backpressure, 1000-message producer/consumer stress, local owner
    cleanup и `SC-RT-312`
  - тот же suite проверяет cancellation task, заблокированной на пустом
    `receive` и полном `send`, сохранение буфера и открытого состояния Channel
  - тот же native suite закрепляет пять одновременно запущенных producers через
    bounded Channel и повторный `run -> wait` с новым handle внутри каждой
    итерации цикла
  - CLI smoke проверяет `check/build/run`, result-bearing task, cooperative stop и
    Channel официальным workflow
  - `tests/task_model_sanitizer.rs` является обязательным TSan gate при
    `SKADI_REQUIRE_TSAN=1`; dedicated CI job не позволяет silently skip проверку
    и использует `setarch x86_64 -R` для стабильного GCC TSan startup
  - native compiler matrix запускает systems project через Linux GCC/Clang и
    Windows MinGW/MSVC
  - `bench_12_systems_pipeline.skd` проверяет совместное использование thread-local
    Memory context и Task/Channel runtime
- experimental Time/Duration coverage
  - `tests/time_model.rs` проверяет literals, overflow, formatter, nominal semantic
    arithmetic, invalid conversions, codegen shape и List/Task/Channel boundaries
  - `tests/codegen_e2e.rs` запускает monotonic measurement и blocking sleep/delay
  - `bench_13_time_budget.skd` входит в native showcase gate
- experimental ByteSize coverage
  - `tests/byte_size_model.rs` проверяет literals, overflow, formatter, nominal
    arithmetic, invalid numeric mixing, dynamic Memory capacity и
    List/Task/Channel boundaries
  - codegen shape закрепляет non-positive runtime guard до `size_t` conversion
  - `bench_14_byte_size_budget.skd` входит в native showcase gate
- experimental Angle coverage
  - `tests/angle_model.rs` проверяет integer/fractional/finite literals,
    formatter, nominal and scalar arithmetic, conversions, invalid mixing,
    codegen и List/Task/Channel boundaries
  - `tests/codegen_e2e.rs` запускает angle arithmetic и trigonometry через native C
  - `bench_15_angle_navigation.skd` входит в native showcase gate
- experimental Vector coverage
  - `tests/vector_model.rs` проверяет точную construction shape, components, арифметику,
    builtins, negative dimension/type rules, formatter, codegen и List/Task/Channel boundaries
  - `tests/codegen_e2e.rs` запускает vector math и zero normalization через native C
  - `bench_16_vector_navigation.skd` входит в native showcase gate
- experimental Canvas coverage
  - `tests/canvas_model.rs` проверяет constructors, палитру, drawing methods,
    resource/borrow negative rules и разделение headless/Window runtime
  - native headless scene закреплена deterministic framebuffer checksum
  - Win32 presenter shape и `gdi32` link flags проверяются без открытия окна в CI
  - `bench_17_canvas_palette.skd` входит в native showcase gate

## 2. Что покрыто частично / что ещё требует углубления

- политика runtime для out-of-range indexing
  - зафиксирована для `v1` как fail-soft (`List` index -> `0`, `Text` index -> `'\0'`)
  - codegen contract tests проверяют форму вспомогательных runtime helpers
- transition surfaces
  - typed periodic `on interrupt` проходит semantic/codegen/runtime; hardware IRQ binding остаётся TODO
  - compound assignments проверяются от parser desugaring до C lowering
  - legacy typed returns принимаются с warning и formatter переводит их в `returns`
  - legacy C-style `for` сохраняет parser/formatter coverage, но имеет обязательный negative semantic gate `SC-SEM-040`
  - ASCII `Char` literals/escapes, invalid forms, typed List и native execution покрыты
  - untyped scalar declarations получают корректный C type; composite declarations без типа отклоняются с `SC-SEM-020`
  - platform hardware binding для interrupts/events остаётся TODO
- task/channel backend/runtime
  - void и `Task(T)` run/wait, `stop`, `stopping`, bounded Channel и cancellation
    blocking `send/receive` реализованы через Win32/pthread backend
  - `send_for/receive_for`, timed Task wait, `timed_out` и
    success/close/stop/timeout precedence покрыты native e2e; `select`, task
    groups и embedded APIs остаются TODO
- structured analysis foundation
  - `tests/analysis_facts.rs` закрепляет task-aware blocking/timed Channel facts,
    timed Task wait, explainable incomplete `when` и subject-linked lifecycle chains
  - CLI action pipeline передаёт facts в TUI Diagnostics / Analysis view,
    который показывает source-order lifecycle выбранного ресурса
- module ergonomics
  - относительный path-import и правила видимости покрыты полноценно
  - path import aliases реализованы; module-name imports и re-export остаются TODO

## 3. Политика для новых фич

Для каждой новой реализованной фичи нужно добавлять:

1. parser test (`parser_smoke`)
2. semantic positive + negative tests (`semantic_smoke`)
3. codegen shape check (`codegen_smoke`)
4. хотя бы один integration scenario (`language_programs` или `codegen_e2e`)

## 4. Инварианты codegen (golden-lite)

Критические маркеры понижения проверяются напрямую в тестах как устойчивые фрагменты, а не как полные snapshot-файлы:

- маркеры `when -> if / else if` (`__when_tmp_*`)
- форма понижения `danger fn` + `on error` (`fn(..., *out)` и `if (call(...) != 0)`)
- runtime hooks для `List/Text/fs/io`
- понижение только statement-level `i++/i--` (`i += 1;` / `i -= 1;`)
