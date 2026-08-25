# skadi-cli

Основной пользовательский интерфейс Skadi `v1.2.0-rc.1`.

- [Установка](../../docs/SKADI_INSTALLATION_RU.md)
- [Быстрый старт CLI](../../docs/SKADI_CLI_QUICK_START_RU.md)
- [Справочник CLI/TUI](../../docs/SKADI_CLI_REFERENCE_RU.md)
- [C ABI и native C](../../docs/SKADI_C_ABI_RU.md)
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
исходные точки останова, `continue`, `step` и `quit`. Остановка показывает
Skadi call stack и базовые scalar-locals. Тот же машинный debug-канал используется
экраном Debug внутри TUI.

`build` и `run` принимают `--target` и `--cc`. C-компилятор является внешней
зависимостью и не устанавливается вместе со Skadi.

Project manifest может подключать проверенные C sources и libraries через
`[native]`; Skadi-граница объявляется bodyless `external fn` с fixed scalar
типами, `external struct` для by-value C-layout и call-scoped `Buffer(T)`.
Pointer/resource ABI пока намеренно не поддерживается.

Секция `[dependencies]` связывает имя локального Skadi package с относительной
директорией, содержащей свой `Skadi.toml`. Такой пакет импортируется как
`import "name/path/file.skd"`; Git/registry resolution и lock-файл пока
отложены.

## TUI

TUI поддерживает:

- dashboard проекта;
- диагностику с кодами и stage;
- structured analysis facts для blocking/timed Channel и timed Task wait,
  incomplete `when` и ownership/resource lifecycle chains с explain/next-action detail;
- отдельный Lifecycle workspace с фильтрами Tasks, Channels, Memory и Resources;
- Debug workspace с исходной позицией, thread ID, call stack, scalar-locals и
  выводом программы;
- `check`, `format`, `build`, `run`, `doctor`;
- редактор каноничных полей `Skadi.toml`, сохраняющий `[dependencies]` и
  `[native]` без потерь;
- выбор target и компилятора для текущего сеанса;
- переключение проектов и просмотр build artifacts.

Основные клавиши:

- `c`, `b`, `r`, `f`, `d` запускают соответствующие действия;
- `m` открывает конфигурацию;
- `o` переключает проект;
- `p`, `e`, `l`, `x`, `h` открывают dashboard, диагностику, lifecycle, debug и help;
- `F5`, `F10`, `F8` запускают/продолжают, выполняют шаг и завершают
  debug-сессию;
- `1`-`5` фильтруют субъекты в Lifecycle workspace;
- `q` завершает TUI.

Пока нет фоновых build-actions и отдельного showcase browser. Debug в TUI
запускается без интерактивного stdin программы; для `input()` и точных source
breakpoints используйте `skadi-cli debug`. Для автоматизации и CI каноническим
остаётся обычный CLI.
