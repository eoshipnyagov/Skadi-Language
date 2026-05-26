# Skadi CLI Quick Start (RU)

Короткий старт для Windows/PowerShell из текущего репозитория.

## 1. Проверка окружения

```powershell
cd D:\YandexDisk\Skadi\v01\tools\skadi-cli
cargo run -- doctor
```

Что важно:
- установлен Rust (`cargo`);
- в `PATH` есть C-компилятор (`gcc` из MinGW или `clang`).

## 2. Справка по командам

```powershell
cargo run -- --help
```

Почему `--`:
- всё, что до `--`, обрабатывает `cargo`;
- всё, что после `--`, передается в `skadi-cli`.

Для TUI-режима:

```powershell
cargo run -- tui
```

Выход из TUI:
- в меню нажать `0` (успешный выход, код `0`).

## 3. Создать новый проект

```powershell
cargo run -- new hello_skadi
cd hello_skadi
```

## 4. Проверить, собрать и запустить

```powershell
cargo run -- check
cargo run -- build
cargo run -- run
```

## 5. Быстрый запуск без `skadi-cli` (через корневой компилятор)

Из корня репозитория:

```powershell
cd D:\YandexDisk\Skadi\v01
cargo run -- --input example_tree.skd --emit-c out.c
cargo run -- --input example_tree.skd --emit-exe tree.exe
```

## 6. Частые проблемы

- `failed to run clang: program not found`:
  установите `clang` или используйте MinGW `gcc` в `PATH`.
- `no working C compiler found`:
  проверьте `gcc --version` и `where gcc`.
- `Unknown option ...`:
  проверьте синтаксис команды через `cargo run -- --help`.

## 7. Минимальный рабочий цикл

1. `doctor`
2. `new <name>` или `init`
3. `check`
4. `build`
5. `run`
