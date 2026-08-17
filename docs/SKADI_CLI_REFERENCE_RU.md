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
| `format [--check] [path ...]` | Форматировать или проверить `.skd` | Нет |
| `build [--target name] [--cc compiler]` | Собрать native binary | Да |
| `run [--target name] [--cc compiler]` | Собрать и запустить | Да |
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

Артефакты находятся в `build/`. `run` передаёт stdout/stderr программы и
сохраняет её exit status.

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
- build/run output;
- doctor/environment;
- project bootstrap;
- `Skadi.toml` config editor;
- help.

Клавиши:

| Клавиша | Действие |
|---|---|
| `c`, `b`, `r`, `f`, `d` | check, build, run, format, doctor |
| `p`, `e`, `m`, `h` | project, errors, manifest config, help |
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
analysis facts. Текущий slice показывает blocking/timed Channel operations,
учитывает task-entry context и `on error`, отмечает `when` без `else` и строит
source-order lifecycle chains для ownership/resources/Task/Memory. Это
информационный workbench, а не новый класс hard errors.

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
