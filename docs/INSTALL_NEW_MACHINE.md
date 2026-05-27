# Skadi: Установка на Новой Машине

Date: 2026-05-27

## 1. Требования

- Git
- Rust toolchain (`cargo`, `rustc`)
- C-компилятор в `PATH`:
  - Windows: `gcc` (MinGW) или `clang` или `cl`
  - Linux/WSL: `gcc` или `clang`
  - macOS: `clang` (Xcode Command Line Tools)

## 2. Клонирование

```bash
git clone git@github.com:eoshipnyagov/Skadi-Language.git
cd Skadi-Language
```

## 3. Установка команды `skadi`

Windows:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install_skadi.ps1
```

Linux/macOS/WSL:

```bash
bash ./scripts/install_skadi.sh
```

## 4. Быстрая проверка

```bash
skadi doctor
skadi new console demo
cd demo
skadi check
skadi build
skadi run
```

Ожидаемый результат: успешные `check/build/run` и вывод `Hello from Skadi!`.

## 5. Если нужно работать через Cargo

```bash
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor
```

`--` обязателен: он отделяет аргументы Cargo от аргументов `skadi-cli`.

## 6. Troubleshooting

1. `gcc/clang not found`
- Установить C-компилятор.
- Проверить: `gcc --version` или `clang --version`.
- Выполнить `skadi doctor`.

2. `failed to read Skadi.toml`
- `check/build/run` запускаются из директории проекта.
- Перейти в папку проекта (`cd demo`) и повторить.

3. `cargo not found`
- Установить Rust через rustup и перезапустить терминал.

4. Сбой сборки на новой ОС
- Запустить:
  - `cargo test -q`
  - `cargo clippy --all-targets --all-features`
  - `cargo clippy --manifest-path tools/skadi-cli/Cargo.toml --all-targets --all-features`
- Сверить с последним успешным CI в GitHub Actions.
