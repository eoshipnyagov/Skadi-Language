# Матрица тестового покрытия (stable `v1.1` + experimental `v1.2`)

Дата: 2026-07-19
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
  - compile-pipeline shape tests покрывают `bench_01..12`
  - native build suite подтверждает `Skadi -> C -> native exe` для `bench_01..12`
  - runtime showcase e2e покрывает:
    - CLI-driven subset `bench_01..05`
    - stable subset `bench_06..09`
    - dedicated full showcase `bench_10_v1_1_toolbox.skd`
    - concurrency showcase `bench_11_task_channel_pipeline.skd`
    - combined systems showcase `bench_12_systems_pipeline.skd`
  - showcase fixtures лежат в `benchmarks/showcase-data/` и используются в script/e2e smoke-path
- experimental memory frontend coverage
  - `tests/memory_model_frontend.rs` проверяет parser/semantic contract для `Memory`, `place in`, `clear`, escape rules и illegal `Memory` usage
  - `tests/memory_model_examples.rs` проверяет self-contained positive examples, большой canonical example, native build/run path, style pitfalls и negative example suite из `examples/memory/`
- experimental task/channel frontend coverage
  - `tests/task_model_frontend.rs` проверяет parser/semantic contract для `Task`, `run`, `wait`, `stop`, `stopping`, `Channel(T)`, `channel(N)`, `send` и `receive`
  - тот же suite проверяет ignored-run hard error, all-path lifecycle,
    task-safe boundaries и value-safe channel messages, включая запрет mutable `List`
  - memory suites проверяют TLS shape и native Win32/pthread isolation active regions
  - `tests/task_model_runtime.rs` проверяет native void и result-bearing tasks,
    typed arguments, scalar/struct/Text result transfer, `stop -> stopping -> wait`,
    bounded FIFO, backpressure, 1000-message producer/consumer stress, local owner
    cleanup и `SC-RT-312`
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

## 2. Что покрыто частично / что ещё требует углубления

- политика runtime для out-of-range indexing
  - зафиксирована для `v1` как fail-soft (`List` index -> `0`, `Text` index -> `'\0'`)
  - codegen contract tests проверяют форму вспомогательных runtime helpers
- runtime semantics для `on interrupt` / `on event`
  - parse-level coverage уже есть, runtime binding остаётся TODO
- task/channel backend/runtime
  - void и `Task(T)` run/wait, `stop`, `stopping` и bounded Channel реализованы через Win32/pthread backend
  - `close`, timeout, cancellation, `select`, task groups и embedded APIs остаются TODO
- module ergonomics
  - относительный path-import и правила видимости покрыты полноценно
  - module-name imports и aliases остаются TODO

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
