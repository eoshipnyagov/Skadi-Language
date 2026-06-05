# Skadi: Быстрый старт CLI (RU)

Короткий старт для нового пользователя Skadi `v1.1`.

Роль этого документа: быстро провести через первый запуск и базовый цикл
работы без подробного разбора всего синтаксиса.

Полный гайд по языку: [Начало работы](getting-started.md)  
Полный справочник CLI/TUI: [Справочник CLI/TUI](cli-reference.md)

## 1. Что нужно заранее

- Rust с `cargo`
- C-компилятор в `PATH`
  - Windows: `gcc` из MinGW-w64, `clang` или `cl`
  - Linux / WSL: `gcc`, `clang` или `cc`
  - macOS: Xcode Command Line Tools (`clang`)

## 2. Канонический пользовательский путь

Для `v1.1` основной интерфейс пользователя - `skadi-cli`.

Низкоуровневый запуск корневого компилятора через `cargo run -- --input ...`
остаётся доступным, но для повседневной работы и документации основной путь -
`skadi-cli`.

## 3. Проверка окружения

В рабочей директории репозитория запустите:

```powershell
skadi-cli doctor
```

`doctor` помогает понять:

- есть ли рабочий C-компилятор для текущей целевой платформы;
- какие компиляторы доступны;
- это ошибка фронтенда Skadi или проблема окружения.

## 4. Если вы работаете из исходников

Если `skadi-cli` уже установлен, этот раздел можно пропустить.

Если вы запускаете команды прямо из репозитория, то до `--` аргументы
обрабатывает `cargo`, а после `--` они передаются в `skadi-cli`.

Пример:

```powershell
cargo run --manifest-path skadi-cli/Cargo.toml -- build --target host --cc gcc
```

## 5. Новый проект

В рабочей директории проекта:

```powershell
skadi-cli new hello_skadi
cd hello_skadi
```

Что создаётся:

- `Skadi.toml`
- `src/main.skd`
- `.gitignore`

## 6. Основной цикл работы

```powershell
skadi-cli check
skadi-cli format
skadi-cli format --check
skadi-cli build
skadi-cli run
```

Рекомендуемый ритм:

- `check` для фронтенда языка;
- `format` для приведения к каноничному стилю;
- `build` для сборки через C-компилятор;
- `run` для полного smoke-пути.

## 7. Инициализация проекта в текущей папке

```powershell
mkdir demo
cd demo
skadi-cli init
```

`init` создаёт отсутствующие базовые файлы, но не ломает уже существующие.

## 8. Явный выбор target и компилятора

```powershell
skadi-cli build --target host --cc gcc
skadi-cli run --target host --cc clang
```

Если `--cc` не задан:

- Windows: `gcc -> clang -> cl`
- Linux / WSL / macOS: `gcc -> clang -> cc`

## 9. Форматирование

```powershell
skadi-cli format
skadi-cli format src/main.skd
skadi-cli format --check
```

`format --check` полезен как локальный release-check перед коммитом.

## 10. Работа через TUI

```powershell
skadi-cli tui
```

`skadi-cli tui` в `v1.1` - полноценный интерактивный путь работы с проектом.

Полезные клавиши:

- `c` - `check`
- `b` - `build`
- `r` - `run`
- `f` - `format`
- `d` - `doctor`
- `m` - config / manifest view
- `o` - открыть или переключить проект
- `g` - создать отсутствующий `entry`-файл из `Config`
- `p` - обзор проекта
- `e` - diagnostics
- `h` - help
- `q` - выход

Что уже умеет TUI:

- обзор проекта;
- экран диагностики;
- build/run view с `stdout/stderr`;
- `doctor` view;
- редактирование `Skadi.toml`;
- настройки сборки для текущего сеанса: `target` и preferred `compiler`;
- создание отсутствующего `entry` файла.

## 11. Типовые ошибки

- Ошибка фронтенда Skadi:

  - проблема в исходнике языка;
  - обычно ловится на `check`.
- Ошибка C-компилятора:

  - проблема с `gcc` / `clang` / `cl`;
  - проверяется через `doctor`.
- Ошибка выполнения программы:

  - Skadi-программа собралась, но уже сработала ошибка при запуске бинаря.

## 12. Что читать дальше

- [Начало работы](getting-started.md) - полный гайд для новичка
- [Справочник языка](language-reference.md) - справочник по языку
- [Справочник CLI/TUI](cli-reference.md) - справочник по CLI и TUI
- [Статус синтаксиса](syntax-status.md) - точный срез текущего синтаксиса
- [Showcase-программы](showcases.md) - витринные программы
