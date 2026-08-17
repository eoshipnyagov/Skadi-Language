# Справочник Skadi CLI и TUI

`skadi-cli` — основной пользовательский toolchain Skadi `v1.2.0-rc.1`.
Обычные команды предназначены для terminal/scripts/CI, а `tui` — для
интерактивной работы.

## Самое важное

Новый проект:

```powershell
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli run
```

Одиночный файл без manifest:

```powershell
skadi-cli quick-run hello.skd
skadi-cli quick-run hello.skd -- first second
```

Обычный цикл:

```powershell
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
```

Проверка среды и интерактивный режим:

```powershell
skadi-cli doctor
skadi-cli tui
```

## Команды

| Команда | Назначение | Нужен C compiler |
|---|---|---:|
| `new <name>` | Создать папку проекта, manifest и entry | Нет |
| `init` | Инициализировать проект в текущей папке | Нет |
| `check` | Imports, lexer, parser, semantic и warnings | Нет |
| `analyze [--json]` | Объяснить blocking и lifecycle behavior | Нет |
| `format [--check] [path ...]` | Форматировать или проверить `.skd` | Нет |
| `build [--target name] [--cc compiler]` | Собрать native binary | Да |
| `run [--target name] [--cc compiler]` | Собрать и запустить | Да |
| `quick-run <file.skd> [-- <args ...>]` | Собрать и запустить один файл без manifest | Да |
| `doctor` | Проверить host/cross toolchains | Нет |
| `target list` | Показать target profiles | Нет |
| `tui` | Открыть full-screen project workflow | Зависит от action |
| `--version` | Показать точную версию | Нет |
| `--help` | Краткая встроенная справка | Нет |

## Проект и manifest

```text
project/
  Skadi.toml
  src/
    main.skd
  build/
```

```toml
[package]
name = "project"
version = "0.1.0"
edition = "v1"

[build]
entry = "src/main.skd"
```

Поддерживаются `name`, `version`, `edition`, `entry`. Config editor внутри TUI
редактирует те же поля.

## `new` и `init`

```powershell
skadi-cli new telemetry
skadi-cli init
```

`new` создаёт отдельную директорию. `init` работает в текущей и не требует
пустой папки, но не должен затирать существующий проект.

## `check`

```powershell
skadi-cli check
```

Команда не запускает C compiler. Ошибки сохраняют стадии и коды:
`Lex`, `Parse`, `Semantic`, `SC-MOD-*`.

## `format`

```powershell
skadi-cli format
skadi-cli format src/main.skd
skadi-cli format --check
```

Без путей форматируется entry текущего проекта. `--check` ничего не меняет и
возвращает ненулевой exit code, если файл неканоничен.

## `analyze`

```powershell
skadi-cli analyze
skadi-cli analyze --json
```

Команда запускает тот же frontend, что `check`, и выводит explainable facts по
Task, Channel, Memory, ownership/resources и неполному `when`. Обычный режим
предназначен для чтения человеком. `--json` использует схему
`skadi.analysis.v1`: каждый факт содержит стабильный ID в рамках исходника,
statement-anchor span, код, уровень, контекст, typed subject, объяснение и
рекомендуемое действие. При frontend-ошибке JSON остаётся валидным, а exit code
остаётся ненулевым. Это automation surface для CI, будущего LSP и других tools.

## `build` и `run`

```powershell
skadi-cli build
skadi-cli build --target host --cc clang
skadi-cli run --target host --cc gcc
```

Ошибки классифицируются по источнику:

- Skadi frontend;
- project/configuration;
- generated C / toolchain;
- runtime execution;
- I/O.

Артефакты находятся в `build/`: executable, generated `.c` и
`<project>.skadi-debug.json`. Debug map использует схему `skadi.debug-map.v1` и
связывает statement ID, исходный `.skd`/line/col и диапазон строк generated C.
Для imports сохраняется реальный файл происхождения, а не только entry. Это
foundation для будущих breakpoints/step/locals, но отдельной команды debugger
пока нет. `run` передаёт stdout/stderr программы и сохраняет её exit status.

## `quick-run`

Короткий путь для примеров и небольших независимых программ:

```powershell
skadi-cli quick-run examples/hello.skd
skadi-cli quick-run tools/inspect.skd -- input.txt --verbose
```

Команда не ищет `Skadi.toml`. Imports по-прежнему разрешаются относительно
исходного файла. Сгенерированные C и executable размещаются во временной папке
и удаляются после завершения. Используется host target, C compiler можно выбрать
через `--cc`, а аргументы программы передаются только после `--`. Установленному
`skadi-cli` Cargo для этого не нужен.

## Targets и doctor

```powershell
skadi-cli target list
skadi-cli doctor
```

`doctor` отдельно показывает readiness host compiler и cross-target candidates,
а также actionable hints. Наличие target profile не означает готовый runtime:
ESP32/FreeRTOS пока [запланирован](embedded.md), но не поддерживается end-to-end.

## TUI

```powershell
skadi-cli tui
```

Экраны:

- project dashboard;
- diagnostics / analysis list и detail с кодом, source location, контекстом и
  рекомендуемым действием;
- отдельный lifecycle workspace для Tasks, Channels, Memory и Resources;
- build/run output;
- путь к generated debug map после build/run;
- doctor/environment;
- project bootstrap;
- `Skadi.toml` config editor;
- help.

Клавиши:

| Клавиша | Действие |
|---|---|
| `c`, `b`, `r`, `f`, `d` | check, build, run, format, doctor |
| `p`, `e`, `l`, `m`, `h` | project, errors, lifecycle, manifest config, help |
| `1`-`5` | Фильтр All/Tasks/Channels/Memory/Resources в Lifecycle |
| `o` | Открыть другой проект |
| `g` | Создать отсутствующий entry из Config |
| `Tab`, `Shift+Tab` | Сменить экран/focus |
| `j/k`, стрелки | Навигация |
| `Enter` | Активировать |
| `q` | Выход |

TUI восстанавливает terminal state при выходе и показывает отдельный fallback
для слишком узкого terminal. Actions пока синхронны; showcase browser и source
editor отсутствуют.

После успешного `check/build` тот же compiler core передаёт TUI structured
analysis facts. Diagnostics оставляет ошибки и предупреждения в общем action
context, а Lifecycle группирует source-order events по стабильному subject ID,
показывает тип субъекта, source span, explain-chain и next action. Текущий slice
покрывает blocking/timed Channel operations, path-sensitive timed Task wait,
task-entry context, `on error`, неполный `when` и
ownership/resources/Task/Memory chains. Это информационный workbench, а не
новый класс hard errors.

## Запуск из исходников

Этот путь нужен только разработчикам toolchain:

```powershell
cargo run -p skadi-cli -- check
```

Установленному пользователю следует писать `skadi-cli check`.

## Что запланировано

- background task execution внутри TUI;
- более подробные per-command help pages;
- package/dependency commands после появления module/package model;
- embedded build/flash workflow после утверждения platform runtime.

Имена текущих команд являются стабильной automation surface; планируемые
возможности не следует закладывать в scripts заранее.
