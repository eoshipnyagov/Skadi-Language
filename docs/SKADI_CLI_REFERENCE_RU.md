# Skadi: Справочник CLI/TUI (RU)

Справочник по `skadi-cli` для Skadi `v1.1`.

Роль этого документа: быть справочником по командам, режимам и поведению
`skadi-cli` и `skadi-cli tui`. Для первого входа удобнее начать с quick start.

Быстрый старт: [Быстрый старт CLI](cli-quick-start.md)  
Полный гайд по языку: [Начало работы](getting-started.md)

Если вы работаете прямо из исходников, те же команды можно запускать через
`cargo run --manifest-path tools/skadi-cli/Cargo.toml -- ...`.

## 1. Роль `skadi-cli`

`skadi-cli` - основной пользовательский интерфейс проекта:

- создаёт и инициализирует проекты;
- проверяет код фронтендом Skadi;
- собирает Skadi -> C -> исполняемый файл;
- запускает программы;
- форматирует `.skd`;
- даёт интерактивный режим работы через TUI.

Для скриптов и CI каноническая поверхность - обычные CLI-команды.  
Для ручной повседневной работы `skadi-cli tui` тоже считается полноценным поддерживаемым путём.

## 2. Команды

### `new <name>`

Создаёт новый проект:

```powershell
skadi-cli new hello_skadi
```

Создаёт:

- `Skadi.toml`
- `src/main.skd`
- `.gitignore`

### `init`

Инициализирует проект в текущей директории:

```powershell
skadi-cli init
```

Полезно, если папка уже существует.

### `check`

Запускает фронтенд языка:

```powershell
skadi-cli check
```

Покрывает:

- lexing;
- parsing;
- semantic analysis.

Не требует рабочего C-компилятора.

### `build`

Собирает проект до исполняемого файла для текущей машины:

```powershell
skadi-cli build
skadi-cli build --target host --cc gcc
```

Флаги:

- `--target <name>`
- `--cc <compiler>`

### `run`

Собирает и запускает проект:

```powershell
skadi-cli run
skadi-cli run --target host --cc clang
```

Стадии ошибки различаются явно:

- фронтенд языка;
- C-компилятор;
- выполнение собранной программы.

### `target list`

Показывает доступные target:

```powershell
skadi-cli target list
```

### `format`

Форматирует `.skd`:

```powershell
skadi-cli format
skadi-cli format src/main.skd
skadi-cli format --check
```

Режимы:

- `format` - переписывает файл в каноничный вид;
- `format --check` - не меняет файл и завершается с ошибкой, если нужно форматирование.

### `doctor`

Проверяет окружение сборки:

```powershell
skadi-cli doctor
```

Полезен для:

- первой настройки машины;
- различения ошибок фронтенда и окружения;
- проверки текущей целевой платформы.

### `tui`

Запускает full-screen интерфейс:

```powershell
skadi-cli tui
```

## 3. Структура проекта

Минимальный проект выглядит так:

```text
hello_skadi/
  Skadi.toml
  src/
    main.skd
  build/
```

Типичный `Skadi.toml`:

```toml
[package]
name = "hello_skadi"
version = "0.1.0"
edition = "v1"

[build]
entry = "src/main.skd"
```

Поддерживаемые каноничные поля:

- `name`
- `version`
- `edition`
- `entry`

## 4. Поведение команд

### Что делает `check`

- читает `Skadi.toml`;
- находит `entry`;
- запускает фронтенд Skadi;
- показывает диагностические сообщения.

### Что делает `build`

- запускает фронтенд языка;
- генерирует C;
- вызывает выбранный C-компилятор;
- кладёт артефакты в `build/`.

### Что делает `run`

- делает `build`;
- запускает собранный бинарь;
- отдаёт `stdout/stderr`.

## 5. Выбор компилятора

Если `--cc` не указан, используется стандартная цепочка перебора.

Windows:

- `gcc`
- `clang`
- `cl`

Linux / WSL / macOS:

- `gcc`
- `clang`
- `cc`

## 6. TUI

`skadi-cli tui` в `v1.1` сфокусирован на работе с проектом:

- обзор проекта;
- diagnostics;
- build/run;
- doctor;
- config editor;
- bootstrap flow.

### Основные клавиши

- `q` - выход
- `c` - check
- `b` - build
- `r` - run
- `f` - format
- `d` - doctor
- `m` - config
- `o` - open / switch project
- `g` - создать отсутствующий `entry` из Config
- `p` - обзор проекта
- `e` - diagnostics
- `h` - help
- `Tab` / `Shift+Tab` - переключение экранов
- `j` / `k` или стрелки - навигация

### Что умеет Config view

- редактировать `name`, `version`, `edition`, `entry`;
- сохранять `Skadi.toml` клавишей `s`;
- показывать `clean/modified` состояние манифеста;
- хранить настройки сборки для текущего сеанса:

  - `target`
  - preferred `compiler`
- создавать отсутствующий `entry` файл клавишей `g`.

### Ограничения текущего TUI

- нет отдельного браузера showcase-программ;
- нет фоновых async-задач;
- не является полноценным редактором исходников;
- для автоматизации и CI каноническим остаётся обычный CLI.

## 7. Рекомендуемый порядок работы

### Только CLI

```powershell
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
```

### CLI и TUI

```powershell
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli tui
```

Дальше внутри TUI:

- правим `Skadi.toml` через `m`;
- при необходимости создаём `entry` через `g`;
- гоняем `check`, `build`, `run`, `format`, `doctor`.

## 8. Для нового пользователя

Если нужен не справочник по командам, а путь "как вообще начать писать на Skadi",
смотри [Начало работы](getting-started.md).
