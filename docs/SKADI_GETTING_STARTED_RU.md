# Skadi: Руководство для новичка (RU)

Роль этого документа: быстро довести нового пользователя до рабочего цикла
`написал -> check -> format -> build -> run`.

Здесь не объясняются основы программирования вроде "что такое цикл". Только
практическая база по текущему состоянию языка и `skadi-cli`.

Если вы работаете прямо из исходников репозитория, те же команды можно запускать
из корня репозитория через `cargo run --manifest-path tools/skadi-cli/Cargo.toml -- ...`.

## 1. Что такое Skadi в этом репозитории

Skadi в текущем репозитории - это рабочий прототип языка с пайплайном:

```text
Skadi source -> lexer -> parser -> semantic -> C codegen -> C compiler -> binary
```

Практически это значит:

- вы пишете `.skd`;
- `skadi-cli` проверяет, форматирует, собирает и запускает код;
- backend сейчас идёт через C-компилятор.

## 2. С чего начать

### Новый проект

В рабочей директории репозитория:

```powershell
skadi-cli new hello_skadi
cd hello_skadi
```

### Базовый цикл

```powershell
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
```

### Интерактивный режим

```powershell
skadi-cli tui
```

## 3. Как устроен проект

Минимальный проект:

```text
hello_skadi/
  Skadi.toml
  src/
    main.skd
  build/
```

Пример `Skadi.toml`:

```toml
[package]
name = "hello_skadi"
version = "0.1.0"
edition = "v1"

[build]
entry = "src/main.skd"
```

Смысл полей:

- `name` - имя проекта
- `version` - версия пакета
- `edition` - версия языкового профиля проекта
- `entry` - точка входа

## 4. Первая программа

```skadi
new Text greeting = concat("Hello", " from Skadi")
output(greeting)

new Float quarter_turn = deg_to_rad(90)
output(quarter_turn)
```

Проверка и запуск:

```powershell
skadi-cli check
skadi-cli run
```

## 5. Объявления и типы

### Объявление без явного типа

```skadi
new x = 10
new name = "Alice"
```

### Объявление с явным типом

```skadi
new Int count = 10
new Float ratio = 0.5
new Bool ok = true
new Text title = "Skadi"
new Path root = "."
```

### Списки

```skadi
new i32 List xs = [1, 2, 3]
new Text List names = ["A", "B"]
new Path List entries = fs.list(".")
```

### Поддерживаемые типы, на которые стоит опираться

Чаще всего:

- `Int`
- `Float`
- `Bool`
- `Char`
- `Text`
- `Path`

Также поддерживаются fixed-width типы:

- `i8`, `i16`, `i32`, `i64`
- `u8`, `u16`, `u32`, `u64`
- `f32`, `f64`

Стилевое правило:

- в обычном коде предпочитай `Int`, `Float`, `Bool`, `Char`, `Text`, `Path`;
- fixed-width типы используй там, где важна разрядность.

Совместимость:

- `bool` и `char` принимаются;
- в витринном стиле предпочтительны `Bool` и `Char`.

## 6. Присваивание

```skadi
new Int total = 0
total = total + 1
```

Инкремент и декремент:

```skadi
new Int i = 0
i++
i--
```

`i++` и `i--` работают как отдельные statements, а не как expression.

## 7. Функции

### Обычная функция

```skadi
fn add(Int a, Int b) Int {
    return a + b
}

new Int result = add(2, 3)
```

### `danger fn`

```skadi
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn safe_div(Int a, Int b) Int {
    if b == 0 {
        return error ZeroDivision
    }

    return a / b
}
```

Важные правила:

- `return error X` работает только в `danger fn`
- для этого нужен `label ErrorCode`
- первый вариант в `ErrorCode` должен быть `Ok`

## 8. `on error`

Если вызывается `danger fn`, можно повесить обработчик:

```skadi
new Int value = safe_div(10, 2) on error {
    output("division failed")
    return
}
```

Или без присваивания:

```skadi
safe_div(10, 0) on error {
    output("division failed")
}
```

`on error` разрешён только на вызовах, которые считаются `danger`.

## 9. Управляющие конструкции

### `if / else`

```skadi
if total > 0 {
    output("positive")
} else {
    output("zero or negative")
}
```

### `while`

```skadi
new Int i = 0
while i < 3 {
    output(i)
    i++
}
```

### `loop`

```skadi
loop {
    pass
    break
}
```

### `for ... in`

```skadi
new i32 List xs = [1, 2, 3]
for item in xs {
    output(item)
}
```

### `iterate ... as ...`

Это каноничный витринный стиль:

```skadi
new i32 List xs = [1, 2, 3]
iterate xs as item {
    output(item)
}
```

### Legacy C-style `for`

Поддерживается, но не считается каноничным стилем:

```skadi
for (i = 0; i < 10; i++) {
    output(i)
}
```

### `when / is / else`

```skadi
when mode {
    is 1 {
        output("one")
    }
    is 2, 3 {
        output("two or three")
    }
    else {
        output("other")
    }
}
```

### `break`, `continue`, `pass`

```skadi
while true {
    continue
    break
}

pass
```

## 10. Списки

### Литерал

```skadi
new i32 List xs = [1, 2, 3]
```

### Индексация

```skadi
new i32 value = xs[1]
```

Текущий контракт `v1`:

- индекс вне диапазона у `List` даёт fail-soft default value;
- это не `on error`.

### `push`

```skadi
xs.push(4)
```

### `pop() on error`

```skadi
new i32 value = xs.pop() on error {
    output("empty list")
    return
}
```

### `len`

```skadi
new Int n = len(xs)
```

## 11. `Text` и `Path`

### Строки

```skadi
new Text t = "weather"
new Int n = len(t)
new Char c = t[0]
```

### Builtins для текста

```skadi
new bool has_station = contains(t, "station")
new Int idx = find(t, "ther")
new Text part = slice(t, 3, 7)
new Text joined = concat("hello", " world")
```

### `Path`

`Path` сейчас ведёт себя как удобное имя для path-oriented текстовых значений.

```skadi
new Path root = "."
new Path full = fs.join(root, "src")
```

## 12. Файлы, аргументы и вывод

```skadi
new Text List cli_args = args()
new Text name = input("name: ")
new Text body = read("in.txt")
new Int ok = write("out.txt", body)
output(body)
```

Поддерживаемые builtins:

- `args()`
- `input(prompt)`
- `read(path)`
- `write(path, text)`
- `output(value)`

## 13. Файловая система

```skadi
new Path root = "."
new Path List entries = fs.list(root)

iterate entries as entry {
    new Path full = fs.join(root, entry)
    if fs.is_dir(full) {
        output(full)
    }
}
```

Поддерживаются:

- `fs.list(path)`
- `fs.join(a, b)`
- `fs.is_dir(path)`

## 14. Struct и методы

```skadi
struct Account {
    Int balance
    Text owner

    fn deposit(Int amount) Int {
        my.balance = my.balance + amount
        return my.balance
    }
}

new Account acc = {balance = 100, owner = "Alice"}
new Int next = acc.deposit(25)
output(acc.balance)
```

Что поддерживается:

- объявление `struct`;
- поля;
- методы;
- `my.field` внутри методов;
- доступ к полю через `obj.field`;
- вызов метода через `obj.method(...)`;
- struct literals.

Поддерживается и field-punning:

```skadi
new Int value = 7
new Text status = "ok"
new Result r = {value, status}
```

## 15. Математика `v1.1`

Константы:

- `PI`
- `TAU`
- `E`
- `EPSILON`

Функции:

- `abs`
- `min`
- `max`
- `clamp`
- `floor`
- `ceil`
- `round`
- `sin`
- `cos`
- `atan2`
- `sqrt`
- `root`
- `deg_to_rad`
- `rad_to_deg`

Пример:

```skadi
new Float heading_deg = 45.0
new Float heading_rad = deg_to_rad(heading_deg)
new Float dx = cos(heading_rad)
new Float dy = sin(heading_rad)
new Float restored_deg = rad_to_deg(atan2(dy, dx))
new Float bounded = clamp(restored_deg, 0.0, 90.0)
output(bounded)
```

## 16. Что ещё есть, но пока не стоит считать завершённой частью языка

### `on interrupt`

Синтаксис уже парсится:

```skadi
on interrupt shutdown {
    output("cleanup")
}
```

Но семантика выполнения этого трека ещё не считается завершённой в `v1.1`.

## 17. Диагностика

Skadi уже старается различать классы ошибок:

- `Lex error`
- `Parse error`
- `Semantic error`

У parse/semantic diagnostics есть коды вида:

- `SC-PARSE-*`
- `SC-SEM-*`

Это важно и для чтения ошибок, и для регрессионных тестов.

## 18. Форматирование

```powershell
skadi-cli format
skadi-cli format --check
```

`format` уже является нормальной частью повседневной работы в `v1.1`.

## 19. TUI

Если не хочется каждый раз работать только командной строкой:

```powershell
skadi-cli tui
```

TUI умеет:

- открыть или переключить проект;
- показывать обзор проекта;
- запускать `check/build/run/format/doctor`;
- показывать diagnostics;
- редактировать `Skadi.toml`;
- создавать отсутствующий `entry`.

## 20. Где смотреть примеры

Showcase-программы:

- `benchmarks/bench_01_tree.skd`
- `benchmarks/bench_02_read_stats.skd`
- `benchmarks/bench_03_find_count.skd`
- `benchmarks/bench_04_sum_ints.skd`
- `benchmarks/bench_05_push_pop.skd`
- `benchmarks/bench_06_struct_account.skd`
- `benchmarks/bench_07_struct_list.skd`
- `benchmarks/bench_08_path_list_helpers.skd`
- `benchmarks/bench_09_math_navigation.skd`

Описание: [Showcase-программы](showcases.md)

## 21. Что читать после этого

- [Справочник языка](language-reference.md) - полный справочник синтаксиса и builtins
- [Справочник CLI/TUI](cli-reference.md) - команды CLI и TUI
- [Статус синтаксиса](syntax-status.md) - точный срез текущего синтаксиса
- [Покрытие тестами](../internal/test-coverage.md) - что реально покрыто тестами
