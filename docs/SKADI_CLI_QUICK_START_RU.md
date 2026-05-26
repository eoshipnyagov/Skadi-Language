# Skadi CLI Quick Start (RU, Cross-Platform)

Быстрый старт для Windows, WSL, Linux и macOS.

## 1. Prerequisites

- Rust toolchain (`cargo`)
- C-компилятор в `PATH`:
  - Windows: `gcc` (MinGW-w64) или `cl` (Visual Studio Build Tools)
  - WSL/Linux: `gcc` или `clang`
  - macOS: Xcode Command Line Tools (`clang`)

## 2. Проверка toolchain

### Windows (PowerShell)

```powershell
cd D:\YandexDisk\Skadi\v01\tools\skadi-cli
cargo run -- doctor
```

### WSL/Linux/macOS (bash)

```bash
cd /path/to/Skadi/v01/tools/skadi-cli
cargo run -- doctor
```

## 3. Почему в `cargo run` нужен `--`

- До `--` аргументы обрабатывает `cargo`.
- После `--` аргументы передаются в программу (`skadi-cli`).

Пример:

```powershell
cargo run -- build --target host --cc gcc
```

Здесь `build --target host --cc gcc` получает именно `skadi-cli`.

## 4. Основной цикл (new/check/build/run)

### Windows (PowerShell)

```powershell
cd D:\YandexDisk\Skadi\v01\tools\skadi-cli
cargo run -- new hello_skadi
cd hello_skadi
cargo run -- check
cargo run -- build
cargo run -- run
```

### WSL/Linux/macOS (bash)

```bash
cd /path/to/Skadi/v01/tools/skadi-cli
cargo run -- new hello_skadi
cd hello_skadi
cargo run -- check
cargo run -- build
cargo run -- run
```

## 5. Явный выбор компилятора

- `--cc <compiler>` принудительно задает C-компилятор:
  - `cargo run -- build --cc gcc`
  - `cargo run -- build --cc clang`
  - `cargo run -- build --cc cl` (Windows host)

Если `--cc` не задан:
- Windows host: `gcc -> clang -> cl`
- Linux/WSL/macOS host: `gcc -> clang -> cc`

## 6. Troubleshooting

- Ошибка: `failed to run <compiler>: ...`
  - Проверьте наличие компилятора: `gcc --version`, `clang --version`, `cl`.
- Ошибка: `no working C compiler for target ...`
  - Запустите `cargo run -- doctor`.
  - Проверьте PATH:
    - Windows: `where gcc`, `where clang`, `where cl`
    - Linux/macOS/WSL: `which gcc`, `which clang`, `which cc`
