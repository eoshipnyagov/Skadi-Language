# Skadi: справка по использованию CLI

Основные пользовательские документы:

- быстрый старт: `SKADI_CLI_QUICK_START_RU.md`
- справочник CLI/TUI: `SKADI_CLI_REFERENCE_RU.md`
- руководство для новичка: `SKADI_GETTING_STARTED_RU.md`

Канонический пользовательский вход:

- `tools/skadi-cli`

Низкоуровневый драйвер компилятора по-прежнему существует:

- `cargo run -- --input program.skd --print-c`
- `cargo run -- --input program.skd --emit-c out.c`
- `cargo run -- --input program.skd --emit-exe out.exe`

Но для обычного пользовательского потока в `v1.1` лучше использовать:

- `new`
- `init`
- `check`
- `build`
- `run`
- `format`
- `doctor`
- `target list`
- `tui`
