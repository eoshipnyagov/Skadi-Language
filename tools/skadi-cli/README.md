# skadi-cli

Основной пользовательский интерфейс Skadi `v1.1`.

Быстрый старт: [Быстрый старт CLI](../../docs/SKADI_CLI_QUICK_START_RU.md)  
Справочник CLI/TUI: [Справочник CLI/TUI](../../docs/SKADI_CLI_REFERENCE_RU.md)  
Гайд для новичка: [Руководство для новичка](../../docs/SKADI_GETTING_STARTED_RU.md)

## Текущее состояние

- Реализовано:

  - `new`, `init`;
  - `check` через реальный frontend языка (`lex / parse / semantic`);
  - `build` (`Skadi -> C -> executable`, поддерживает `--target` и `--cc`);
  - `run` (`build + execute`, поддерживает `--target` и `--cc`);
  - `format` на текущем `v1`-подмножестве;
  - `target list`.
- `tui`:

  - полноэкранный режим работы с проектом;
  - обзор, diagnostics, build/run, doctor, bootstrap, help;
  - редактор каноничных полей `Skadi.toml` (`name`, `version`, `edition`, `entry`);
  - настройки сборки для текущего сеанса: `target` и предпочтительный компилятор;
  - переключение проектов, снимок манифеста и панели артефактов сборки.
- Планируется:

  - дополнительные target/toolchain цепочки;
  - фоновые задачи внутри TUI;
  - отдельный браузер showcase-программ внутри TUI.

## Примеры использования

```powershell
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- help
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- check
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- build
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- build --target host --cc gcc
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- run
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- format
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- format src/main.skd
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- tui
```

Рекомендуемый порядок работы:

```powershell
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- new hello_skadi
cd hello_skadi
cargo run --manifest-path ..\tools\skadi-cli\Cargo.toml -- check
cargo run --manifest-path ..\tools\skadi-cli\Cargo.toml -- build
cargo run --manifest-path ..\tools\skadi-cli\Cargo.toml -- run
```

Интерактивный режим:

```powershell
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- tui
```

Внутри `tui`:

- `c` запускает `check`;
- `b` запускает `build`;
- `r` запускает `run`;
- `f` запускает `format`;
- `d` открывает или обновляет `doctor`;
- `m` открывает редактор манифеста и конфигурации;
- `o` открывает или переключает текущий проект;
- `p`, `e`, `h` переключают обзор проекта, диагностику и справку;
- `g` в `Config` создаёт текущий `entry`-файл и родительские папки;
- `target` и `compiler` в `Config` влияют на `build/run` в рамках текущего сеанса.

Текущие ограничения TUI:

- пока нет отдельного браузера showcase-программ;
- пока нет фоновых задач;
- для автоматизации и CI каноническим остаётся обычный CLI.

