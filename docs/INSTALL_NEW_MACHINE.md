# Skadi: Установка на Новой Машине

Date: 2026-05-27

## 1. Что нужно заранее

- Git
- Rust toolchain (`cargo`, `rustc`)
- C-компилятор в `PATH`:
  - Windows: `gcc` (MinGW) или `clang` или `cl`
  - Linux/WSL: `gcc` или `clang`
  - macOS: `clang` (Xcode Command Line Tools)

## 2. Клонирование проекта

```bash
git clone git@github.com:eoshipnyagov/Skadi-Language.git
cd Skadi-Language
```

## 3. Проверка ядра компилятора

```bash
cargo test -q
cargo clippy --all-targets --all-features
```

## 4. Проверка CLI

```bash
cargo clippy --manifest-path tools/skadi-cli/Cargo.toml --all-targets --all-features
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor
```

## 5. Smoke-проверка end-to-end

```bash
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- new console demo
cd demo
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- check
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- build
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- run
```

Ожидаемый результат: успешные `check/build/run` и вывод `Hello from Skadi!`.

## 6. Опционально: команда `skadi` в PATH

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install_skadi.ps1
```

Linux/macOS/WSL:

```bash
bash ./scripts/install_skadi.sh
```

## 7. Troubleshooting

1. Ошибка `gcc/clang not found`
- Установить C-компилятор и проверить `gcc --version` или `clang --version`.
- Выполнить `doctor`:
  - `cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor`

2. Ошибка `failed to read Skadi.toml`
- `check/build/run` выполняются из директории проекта.
- Перейти в папку проекта (`cd demo`) и повторить команду.

3. Ошибка `cargo not found`
- Установить Rust через rustup и перезапустить терминал.

4. Ошибка сборки на новой ОС
- Запустить:
  - `cargo test -q`
  - `cargo clippy --all-targets --all-features`
  - `cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor`
- Сверить результаты с CI в GitHub Actions.
