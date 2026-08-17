# skadi-cli

Основной пользовательский интерфейс Skadi `v1.2.0-rc.1`.

- [Установка](../../docs/SKADI_INSTALLATION_RU.md)
- [Быстрый старт CLI](../../docs/SKADI_CLI_QUICK_START_RU.md)
- [Справочник CLI/TUI](../../docs/SKADI_CLI_REFERENCE_RU.md)
- [Руководство для новичка](../../docs/SKADI_GETTING_STARTED_RU.md)

## Основной workflow

После установки команды выполняются напрямую:

```bash
skadi-cli doctor
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli format --check
skadi-cli build
skadi-cli run
```

Для одиночного `.skd` без manifest и постоянных build-артефактов:

```bash
skadi-cli quick-run hello.skd
skadi-cli quick-run hello.skd -- first second
```

`--` отделяет аргументы `skadi-cli` от аргументов запускаемой программы. Cargo
после установки не нужен; для native-сборки по-прежнему нужен host C compiler.

Интерактивный режим:

```bash
skadi-cli tui
```

Cargo нужен только для работы с checkout репозитория:

```bash
cargo run -p skadi-cli -- --version
cargo run -p skadi-cli -- check
```

## Команды

- `new`, `init` создают проект;
- `check` запускает `lex / parse / semantic`;
- `analyze [--json]` объясняет blocking/lifecycle facts и выдаёт стабильный JSON для tools/CI;
- `format` форматирует проект или проверяет стиль через `--check`;
- `build` выполняет `Skadi -> C -> executable`;
- `run` собирает и запускает программу;
- `debug` запускает opt-in debug build с breakpoints по `.skd`, `continue` и `step`;
- `quick-run` собирает и запускает один `.skd` без `Skadi.toml`;
- `doctor` проверяет host и cross toolchains;
- `target list` показывает поддерживаемые target profiles;
- `tui` открывает полноэкранный интерфейс проекта.

`build` также создаёт `<project>.skadi-debug.json` со statement-level mapping
между исходными `.skd` и generated C. Sidecar уже учитывает imports и является
основой Skadi-level debugger. Команда `debug [-b file.skd:line]` уже поддерживает
исходные точки останова, `continue`, `step` и `quit`; locals, call stack и
debug-сессия внутри TUI пока не готовы.

`build` и `run` принимают `--target` и `--cc`. C-компилятор является внешней
зависимостью и не устанавливается вместе со Skadi.

## TUI

TUI поддерживает:

- dashboard проекта;
- диагностику с кодами и stage;
- structured analysis facts для blocking/timed Channel и timed Task wait,
  incomplete `when` и ownership/resource lifecycle chains с explain/next-action detail;
- отдельный Lifecycle workspace с фильтрами Tasks, Channels, Memory и Resources;
- `check`, `format`, `build`, `run`, `doctor`;
- редактор каноничных полей `Skadi.toml`;
- выбор target и компилятора для текущего сеанса;
- переключение проектов и просмотр build artifacts.

Основные клавиши:

- `c`, `b`, `r`, `f`, `d` запускают соответствующие действия;
- `m` открывает конфигурацию;
- `o` переключает проект;
- `p`, `e`, `l`, `h` открывают dashboard, диагностику, lifecycle и help;
- `1`-`5` фильтруют субъекты в Lifecycle workspace;
- `q` завершает TUI.

Пока нет фоновых задач и отдельного showcase browser. Для автоматизации и CI
каноническим остаётся обычный CLI.
