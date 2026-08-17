# Skadi: технический справочник проекта

Дата сверки: 2026-07-29
Состояние: stable toolchain base `v1.1` и experimental systems line `v1.2`

Этот документ описывает фактическую архитектуру репозитория. Детали конкретного
синтаксиса находятся в [справочнике языка](../user/language-reference.md), а
точный уровень поддержки - в [статусе синтаксиса](../user/syntax-status.md).

## 1. Основной pipeline

Официальный пользовательский путь проходит через `skadi-cli`:

```text
Skadi project / Skadi.toml
    -> path-import loading and visibility checks
    -> lexer
    -> parser / AST
    -> semantic analysis and style warnings
    -> C codegen plus required runtime slices
    -> selected host C compiler
    -> native binary
```

Команды `check`, `build` и `run` используют один frontend pipeline. TUI вызывает
те же внутренние actions, а не запускает CLI-команды через shell.

Корневой `src/main.rs` остаётся низкоуровневым compiler driver для разработки;
он не является рекомендуемым пользовательским интерфейсом.

## 2. Workspace

Основные части репозитория:

- `src/` - compiler core как Rust library и low-level driver;
- `tools/skadi-cli/` - project CLI, TUI, target/toolchain orchestration;
- `tests/` - parser, semantic, codegen, native e2e и systems runtime suites;
- `benchmarks/` - поддерживаемые showcase-программы;
- `examples/` - positive/negative language и systems examples;
- `docs/`, `docs-en/` - исходники RU/EN документации;
- `scripts/` - docs/showcase automation;
- `.github/workflows/` - cross-platform CI, TSan и GitHub Pages.

Generated `.docs-build/`, `site/`, `target/`, `.exe`, `.vsix` и C artifacts не
являются исходниками проекта.

## 3. Compiler core

### `src/common_types.rs`

Хранит `TokenKind` и `Token`. Часть слов уже зарезервирована lexer-ом для future
surface; наличие token kind само по себе не означает parser/runtime support.

### `src/lexer/`

- `core.rs` реализует токенизацию, позиции, comments, literals, keywords и operators;
- `structures.rs` содержит `LexError` и lexical helpers;
- `mod.rs` предоставляет публичный `lex`.

Lexer возвращает структурированную диагностику `SC-LEX-*` и не должен panic на
пользовательском вводе.

### `src/ast_nodes.rs`

Определяет `Program`, `Statement`, `Expression`, block/function/struct nodes и
location metadata. Текущий AST включает core language, Memory, Task/Channel и
Duration literals.

### `src/parser/`

- `mod.rs` выбирает statement parser и формирует `Program`;
- `statements.rs` разбирает declarations, control flow, structs, error flow,
  Memory и Task/Channel surface;
- `expressions.rs` реализует precedence parsing, calls, indexing, list/struct
  literals, `run/wait/stopping`, Duration, ByteSize и Angle unit literals.

Parser diagnostics используют семейство `SC-PARSE-*`. Зарезервированные, но не
реализованные формы должны завершаться явной parse error, а не частичным AST.

### `src/semantic_analysis.rs`

Semantic layer отвечает за:

- lexical scopes, запрет shadowing и use-before-definition;
- типы declarations, assignments, calls и return paths;
- `danger fn`, `ErrorCode` и допустимые формы `on error`;
- structs, hidden fields, methods и `my`;
- List/Text/I/O/math builtin signatures;
- Memory capability, lifetime, escape и use-after-clear rules;
- линейный Task lifecycle и task-safe boundaries;
- Channel value-safe payload и owner restrictions;
- nominal `Time/Duration`, `ByteSize` и `Angle` arithmetic;
- bounded `Vec2/Vec3/Vec4` construction, fields, arithmetic и math builtins;
- style warnings для legacy/canonical forms.

Ошибки относятся к `SC-SEM-*`; warning policy не превращает стиль в hard error.

### `src/builtins.rs`

Единый registry имён, arity и categories для collection/text, filesystem, I/O,
math и time builtins. Parser/semantic/codegen не должны поддерживать разные
несогласованные списки публичных builtins.

### `src/formatter.rs`

Печатает канонический поддерживаемый синтаксис. Formatter работает через AST,
используется CLI-командами `format` и `format --check` и пока считается
переходной поверхностью для новых конструкций.

### `src/codegen/`

`codegen/c.rs` понижает проверенный AST в C и добавляет только необходимые
runtime helpers. Реализованы:

- scalar/fixed-width values, functions, control flow и error flow;
- structs, fields, methods и generated cleanup;
- typed List, Text, filesystem и I/O runtime;
- `math.h` lowering;
- fixed/growing/child/root-static Memory regions и thread-local active region;
- explicit ownership transfer и typed cleanup для current linear resources;
- Win32/pthread Task runtime и typed results;
- bounded blocking Channel с mutex/condition variables;
- monotonic Time/Duration runtime на Win32/POSIX;
- nominal ByteSize/Angle lowering и bounded Vec2/Vec3/Vec4 helpers.

Подробная карта representation находится в [границах Skadi -> C](to-c-scope.md).

### `src/diagnostics.rs`

Содержит stage-aware форматирование `Lex`, `Parse`, `Semantic` diagnostics.
Tooling stages добавляют собственные `SC-MOD`, `SC-CG`, `SC-CC` и runtime codes.

## 4. `skadi-cli`

### `actions.rs`

Внутренний action layer возвращает структурированные `CheckResult`,
`BuildResult`, `RunResult`, `DoctorReport`, `FormatResult`, `ProjectSummary` и
классифицированные `ActionError`. Его используют и обычные команды, и TUI.

### `pipeline.rs`

Добавляет project-level возможности поверх compiler core:

- recursive relative path imports;
- cycle/missing-file diagnostics;
- direct-import-only visibility;
- public symbol collision detection;
- `local` isolation и `module.symbol` qualification;
- line-origin table для раскрытого multi-file source;
- statement-level `Skadi -> generated C` mapping и sidecar
  `skadi.debug-map.v1`;
- вызов host/cross C toolchain.

### `project.rs`

Работает с `Skadi.toml`, project bootstrap, entry/build directories и config
editor persistence.

### `targets.rs`

Хранит target profiles, compiler candidates, platform hints и output naming.
Текущий release gate ориентирован на desktop host compilers; embedded profiles
не означают готовый ESP32/RTOS runtime.

### `tui.rs`

Full-screen `ratatui`/`crossterm` application: dashboard, diagnostics,
build/run, doctor, project bootstrap, config editor и help. Долгие actions пока
синхронны; shell-out к собственным CLI-командам не используется.

Целевое развитие TUI - не перенос semantic logic в event loop, а представление
общего structured analysis engine. Тот же набор facts должен обслуживать CLI,
TUI, CI и будущий LSP. Планируемые views: ownership/resource lifecycle,
Task/Channel state, Memory regions, explain-chain diagnostics и source-level
debugger.

Первый debugger строится поверх C pipeline. Lexer сохраняет start location
токена, C-emitter вставляет `SK-STMT` markers и opt-in compiler probes, import
pipeline восстанавливает исходный `.skd`, а `build` пишет portable JSON sidecar.
CLI уже разрешает breakpoints `file.skd:line` и поддерживает `continue`, `step`
и `quit`. Probes между задачами сериализуются, поэтому остановка кооперативная:
это ещё не нативный stop-the-world debugger. Skadi type metadata, locals, call
stack и TUI debug session остаются впереди. GDB/LLDB допустимы как нижний native
layer; собственный machine debugger не является целью.

## 5. Реализованные уровни языка

### Stable base `v1.1`

- core declarations, expressions и control flow;
- functions, `danger fn`, `ErrorCode`, `on error`;
- structs/methods, Text/Path/List, I/O/filesystem;
- relative imports, visibility и qualification;
- math core;
- CLI/TUI, formatter, diagnostics и docs/showcase workflow.

### Experimental line `v1.2`

- bounded Memory runtime с fixed/grow/child/static regions;
- call-scoped `view`/`direct` и explicit resource `move`;
- native Task/Channel MVP на Win32/pthread;
- nominal Time/Duration и monotonic runtime;
- реализованные bounded milestones: Time/Duration, ByteSize, Angle и Vec2/Vec3/Vec4;
- переходные формы закрыты явными diagnostics и compatibility policy.

Experimental означает незамороженный API, а не frontend-only scaffold: текущие
Memory, ownership, Task/Channel, Time/Duration, ByteSize, Angle и Vector slices
исполняются end-to-end.

## 6. Test architecture

Основные уровни доказательства:

- `lexer_smoke`, `parser_smoke`, `parser_negative`;
- `semantic_smoke`, specialized Memory/Task/Time frontend suites;
- `formatter_smoke`;
- `codegen_smoke` и `codegen_golden`;
- `language_programs`, `conformance_suite`, `edge_matrix`;
- `codegen_e2e` с реальным C compiler и runtime execution;
- `showcase_programs` и `showcase_builds` для всех поддерживаемых showcases;
- `memory_model_examples`, `task_model_runtime`, `task_model_sanitizer`.

Новая языковая фича должна иметь parser positive/negative, semantic
positive/negative, codegen shape и user-visible native e2e, если у неё есть
runtime behavior.

## 7. CI и платформы

CI проверяет:

- rustfmt и clippy workspace;
- Rust test suites на Windows, Linux и macOS;
- native showcase builds;
- GCC/Clang/MinGW/MSVC generated-C matrix;
- обязательный GCC ThreadSanitizer job для concurrency runtime;
- strict RU/EN MkDocs build и GitHub Pages.

Локальный успешный host build не заменяет remote matrix для platform runtime.

## 8. Documentation/tooling

`scripts/sync_docs_site.py` формирует `.docs-build/` из Markdown-источников,
`mkdocs build --strict` строит HTML, а `check_docs_consistency.py` проверяет
переносимые пути, route sources, CLI surface и наличие публичных builtins в
справочнике языка.

Подсветку `.skd` поддерживают VS Code grammar и Pygments lexer для документации.
При добавлении syntax surface оба слоя обновляются вместе с formatter и docs.

## 9. Явные границы

Пока не являются текущей реализованной поверхностью:

- hardware `on interrupt` backends;
- module name imports/re-exports;
- Channel `select`, task groups и async/await;
- shared mutable state model;
- ESP32/FreeRTOS и другие embedded runtime backends;
- Canvas events/text/images, Matrix2D, generic units и operator overloading;
- package manager и dependency resolution;
- готовый embedded/ESP32 runtime и flash workflow;
- non-Windows и embedded Canvas presentation backends.

Актуальный порядок работ фиксируется в [плане v1.2](v1-2-plan.md), а найденные
расхождения между design-документами и реализацией собраны в
[аудите внутренней документации](internal-docs-audit.md).
