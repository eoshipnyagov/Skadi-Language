# Skadi: Быстрый старт CLI (RU)

Короткий старт для пользователя Skadi `v1.2.0-rc.1`.

Роль этого документа: быстро провести через первый запуск и базовый цикл
работы без подробного разбора всего синтаксиса.

Полный гайд по языку: [Начало работы](getting-started.md).

Полный справочник CLI/TUI: [Справочник CLI/TUI](cli-reference.md).

Установка: [Установка Skadi CLI](installation.md)

## 1. Что нужно заранее

- установленный `skadi-cli`
- C-компилятор в `PATH`
  - Windows: `gcc` из MinGW-w64, `clang` или `cl`
  - Linux / WSL: `gcc`, `clang` или `cc`
  - macOS: Xcode Command Line Tools (`clang`)

## 2. Канонический пользовательский путь

Основной интерфейс пользователя — установленная команда `skadi-cli`.

Низкоуровневый запуск корневого компилятора через `cargo run -- --input ...`
остаётся доступным, но для повседневной работы и документации основной путь -
`skadi-cli`.

Для одного небольшого файла проект создавать не нужно:

```powershell
skadi-cli quick-run hello.skd
```

Команда использует host C compiler, но после установки `skadi-cli` не требует
Cargo или `Skadi.toml`. Аргументы программы указываются после `--`:

```powershell
skadi-cli quick-run hello.skd -- first second
```

## 3. Проверка окружения

В рабочей директории запустите:

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
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- build --target host --cc gcc
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
skadi-cli debug --break src/main.skd:1
```

После `build` в `build/` лежат executable, generated C и
`<project>.skadi-debug.json`. Последний нужен tooling/debugger и обычно не
редактируется вручную.

Рекомендуемый ритм:

- `check` для фронтенда языка;
- `format` для приведения к каноничному стилю;
- `build` для сборки через C-компилятор;
- `run` для полного smoke-пути.
- `debug` для интерактивной остановки и пошагового выполнения по `.skd`.

Без `--break` debugger остановится на первом исполняемом statement. В сессии
доступны `continue` (`c`), `step` (`s`) и `quit` (`q`).

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

`skadi-cli tui` — полноценный интерактивный путь работы с проектом.

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
- `e` - diagnostics и structured analysis
- `l` - lifecycle workspace для Tasks, Channels, Memory и Resources
- `h` - help
- `q` - выход

Что уже умеет TUI:

- обзор проекта;
- экран diagnostics с action history и объяснением следующего действия;
- отдельный lifecycle workspace с typed subjects, source locations и
  фильтрами `1`-`5`;
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
