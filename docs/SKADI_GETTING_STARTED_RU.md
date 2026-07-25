# Skadi: Руководство для новичка (RU)

Роль этого документа: быстро довести нового пользователя до рабочего цикла
`написал -> check -> format -> build -> run`.

Здесь не объясняются основы программирования вроде "что такое цикл". Только
практическая база по текущему состоянию языка и `skadi-cli`.

Сначала установите готовый `skadi-cli` по
[инструкции установки](installation.md). Запуск через
`cargo run -p skadi-cli -- ...` нужен только при разработке из checkout
репозитория.

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

В выбранной рабочей директории:

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

new Angle quarter_turn = 90deg
output(rad_to_deg(quarter_turn))
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

Без явного типа разрешены скалярные значения: числа, `Bool`, `Char`, `Text`,
`Time`, `Duration`, `ByteSize` и `Angle`. Для `List`, структур, векторов, Task,
Channel и других составных значений тип обязателен; иначе проверка завершится
диагностикой `SC-SEM-020`.

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
fn add(Int a, Int b) returns Int {
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

danger fn safe_div(Int a, Int b) returns Int {
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
new Int value = 0
value = safe_div(10, 2) on error {
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

Такую форму можно встретить в старых исходниках:

```skadi
for (i = 0; i < 10; i++) {
    output(i)
}
```

Parser и formatter понимают её для совместимости, но semantic-анализ отклоняет
с `SC-SEM-040`: backend `v1.2` её не исполняет. В новом коде используйте
`iterate ... as ...` или `for ... in ...`.

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
new i32 value = 0
value = xs.pop() on error {
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

    fn deposit(Int amount) returns Int {
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

### Видимость

Объявления публичны по умолчанию. Для локальных символов и скрытых полей есть
`local` и `hide`:

```skadi
local struct Session {
    hide Text token
    Text name
}

local fn normalize(Text value) returns Text {
    return value
}
```

`hide`-поле доступно методам своей структуры, но не внешнему коду. Shadowing
локальных имён запрещён.

## 14.1 Несколько файлов

Импорт указывается относительным путём от текущего `.skd`-файла:

```skadi
import "./math_utils.skd"

new Int value = math_utils.add(2, 3)
```

Имя перед точкой берётся из имени файла без `.skd`. Публичные символы прямого
импорта также доступны без квалификации. Транзитивные импорты не видны, а
module-name imports и aliases пока не реализованы.

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
new Angle heading = 45deg
new Float dx = cos(heading)
new Float dy = sin(heading)
new Angle restored = atan2(dy, dx)
new Float restored_deg = rad_to_deg(restored)
new Float bounded = clamp(restored_deg, 0.0, 90.0)
output(bounded)
```

## 16. Экспериментальный systems-слой `v1.2`

Stable base `v1.1` не требует этих возможностей, но текущая ветка разработки уже
позволяет проверять и запускать их на Windows и POSIX host.

### `Angle`

```skadi
new Angle heading = 45deg
new Angle correction = 0.25rad
new Angle target = heading + correction
new Float x = cos(target)
new Float y = sin(target)
new Float measured_degrees = rad_to_deg(atan2(y, x))
```

`Angle` - nominal `f64`-тип с внутренним представлением в radians. Литералы
`deg/rad` пишутся слитно; `deg_to_rad` и `atan2` возвращают `Angle`, а
`rad_to_deg` явно возвращает числовые градусы. Полный контракт: [Углы](angle.md).

### `Vec2`, `Vec3`, `Vec4`

```skadi
new Vec2 position = {x = 2.0, y = 1.0}
new Vec2 target = {x = 8.0, y = 9.0}
new Vec2 heading = normalize(target - position)
new Float route_length = distance(position, target)
output(heading.x)
output(route_length)
```

Векторы используют точный набор компонентов `x/y[/z/w]`, хранят их как `f64`
и поддерживают bounded арифметику и функции `dot`, `length`, `length_sq`,
`normalize`, `distance`, `distance_sq`; `cross` определён только для `Vec3`.
Полный контракт: [Векторы](vectors.md).

### `Time` и `Duration`

```skadi
new Time started_at = now()
sleep(5ms)
new Duration measured = elapsed(started_at)
new Bool completed = measured >= 5ms
output(completed)
```

`Time` является точкой monotonic clock, а `Duration` - отдельным nominal-типом.
Поддерживаются целые literals `ms`, `s`, `min`; обычный `Int` не преобразуется в
`Duration` неявно. Полный контракт: [Время и длительности](time-duration.md).

### `ByteSize`

```skadi
new ByteSize payload = 2kb
new ByteSize capacity = payload + 512b
Memory scratch_memory = memory(capacity)
```

`ByteSize` - отдельный nominal-тип количества байтов. Поддерживаются целые
слитные literals `b`, `kb`, `mb`, `gb` с бинарными множителями. Значения можно
складывать, вычитать и сравнивать между собой, но нельзя неявно смешивать с
`Int/Float`. Полный контракт: [Размеры памяти](byte-size.md).

### `Memory`

```skadi
Memory scratch_memory = memory(16kb) on error {
    output("memory allocation failed")
}

place in scratch_memory {
    new Text preview_text = read("input.txt")
    output(preview_text)
} on error {
    scratch_memory.clear()
    output("memory region overflow")
}

scratch_memory.clear()
```

`Memory` - fixed-capacity region и capability handle. Значения с динамическими
данными из локальной region нельзя переносить за её lifetime.

### `Task` и `Channel(T)`

```skadi
fn produce(Channel(Int) values) {
    values.send(42)
}

Channel(Int) values = channel(1)
Task producer_task = run produce(values)
new Int value = values.receive()
wait producer_task
output(value)
```

`Task` использует native Win32/pthread backend, а `Channel(T)` - bounded blocking
FIFO. Каждый owning task handle должен завершиться ровно одним `wait`; `stop`
только публикует кооперативный запрос и не заменяет `wait`.

Подробные ограничения и multi-worker patterns описаны в
[руководстве по многопоточности](concurrency.md).

### `on interrupt`

Синтаксис сохраняется на parse/format уровне:

```skadi
on interrupt shutdown {
    output("cleanup")
}
```

`check` намеренно возвращает `SC-SEM-040`, потому что runtime binding ещё нет.
Такой блок нельзя считать исполняемым кодом текущей версии.

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

`format` является нормальной частью повседневной работы.

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

Для быстрого поиска маленьких рецептов по операторам, ошибкам, структурам,
модулям, I/O, math и systems surface используйте
[Короткие примеры Skadi](language-examples.md).

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
- `benchmarks/bench_10_v1_1_toolbox.skd`
- `benchmarks/bench_11_task_channel_pipeline.skd`
- `benchmarks/bench_12_systems_pipeline.skd`
- `benchmarks/bench_13_time_budget.skd`
- `benchmarks/bench_14_byte_size_budget.skd`
- `benchmarks/bench_15_angle_navigation.skd`
- `benchmarks/bench_16_vector_navigation.skd`

Описание: [Showcase-программы](showcases.md)

Для `Task`, `Channel(T)`, нескольких параллельных workers, повторного запуска и
текущего статуса ESP32 смотри [Многопоточность в Skadi](concurrency.md).

Для быстрых и воспроизводимых smoke-запусков используйте данные из
`benchmarks/showcase-data/`: текстовый fixture для file/text showcase и
мини-дерево каталогов для directory/path showcase.

## 21. Что читать после этого

- [Справочник языка](language-reference.md) - полный справочник синтаксиса и builtins
- [Короткие примеры](language-examples.md) - небольшие рецепты по отдельным конструкциям
- [Справочник CLI/TUI](cli-reference.md) - команды CLI и TUI
- [Многопоточность](concurrency.md) - Task/Channel, lifecycle и платформы
- [Время и длительности](time-duration.md) - `Time`, `Duration`, unit literals и runtime
- [Размеры памяти](byte-size.md) - `ByteSize`, бинарные unit literals и `memory(ByteSize)`
- [Углы](angle.md) - `Angle`, literals `deg/rad` и trigonometry contract
- [Векторы](vectors.md) - `Vec2/Vec3/Vec4`, value semantics и vector math
- [Статус синтаксиса](syntax-status.md) - точный срез текущего синтаксиса
- [Покрытие тестами](../internal/test-coverage.md) - что реально покрыто тестами
