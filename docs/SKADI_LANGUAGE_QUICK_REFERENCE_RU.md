# Skadi: быстрая справка по языку

Это полный компактный каталог публичных языковых форм текущего компилятора.
Он предназначен и как памятка, и как карта для сверки дизайна с реализацией.

Статусы:

- **Stable** — рабочая база `v1.1`;
- **Experimental** — исполняемый `v1.2` MVP с незамороженным API;
- **Compatibility** — читается для совместимости, но не рекомендуется;
- **Reserved** — распознаётся только частью frontend и не компилируется;
- **Future** — описано в планах, но отсутствует в языке.

## Файлы и объявления

Обобщённые шаблоны typed declaration: `new Type name = expression` и
`new ElementType List name = [values]`.

| Форма | Пример | Статус | Назначение |
|---|---|---:|---|
| Строчный комментарий | `// note` | Stable | До конца строки |
| Блочный комментарий | `/* note */` | Stable | Многострочный комментарий |
| Вывод типа | `new count = 10` | Stable | Только для скалярных значений |
| Явный тип | `new Int count = 10` | Stable | Обязателен для composite values |
| Список | `new Int List ids = [1, 2]` | Stable | Тип элемента записывается перед `List` |
| Присваивание | `count = count + 1` | Stable | Изменение существующего значения |
| Compound assignment | `count += 1` | Compatibility | Formatter раскрывает в обычное присваивание |
| Инкремент/декремент | `count++`, `count--` | Stable | Только statement |
| `fixed` / `const` | `fixed Int n = 1` | Reserved | Lexer token без statement semantics |
| `;` и `:` | Punctuation tokens | Reserved вне legacy C-style `for`; statements разделяются строками |

## Типы и литералы

| Категория | Типы или формы | Статус | Примечание |
|---|---|---:|---|
| Целые | `Int`, `i8/i16/i32/i64`, `u8/u16/u32/u64` | Stable | Semantic family `Int`; C layout следует аннотации |
| Вещественные | `Float`, `f32`, `f64` | Stable | Semantic family `Float` |
| Логические | `Bool`, alias `bool`; `true`, `false` | Stable | Канонически `Bool` |
| Символ | `Char`, alias `char`; `'a'`, `'\n'` | Stable | Только ASCII и поддержанные escapes |
| Текст/путь | `Text`, `Path`; `"hello"` | Stable | `Path` использует text representation |
| Список | `Int List`, `[1, 2, 3]` | Stable | `List(T)` не является синтаксисом типа |
| Пользовательские данные | `struct Name`, `{x = 1}` | Stable | Поддерживается field punning `{x, y}` |
| Время | `Time`, `Duration`; `5ms`, `2s`, `3min` | Experimental | Magnitude должна быть целой |
| Размер памяти | `ByteSize`; `64b`, `4kb`, `8mb`, `1gb` | Experimental | Бинарные множители |
| Угол | `Angle`; `90deg`, `0.25rad` | Experimental | Хранение в radians |
| Векторы | `Vec2`, `Vec3`, `Vec4` | Experimental | Компоненты `f64` |
| Задача | `Task`, `Task(Int)` | Experimental | Линейный owning handle |
| Канал | `Channel(Int)` | Experimental | Value-safe message type |
| Регион | `Memory` | Experimental | Capability, не обычное значение |

Escapes `Char`: `\n`, `\r`, `\t`, `\0`, `\'`, `\\`.

## Операторы и выражения

| Группа | Формы | Результат/правило |
|---|---|---|
| Арифметика | `+ - * / % ^` | Числовые правила; specialized types имеют отдельную матрицу |
| Целочисленные слова | `div`, `mod` | Целочисленные операции |
| Сравнение | `== != < > <= >=` | `Bool` |
| Логика | `and or xor not` | Канонические word operators |
| Symbolic logic | `&& \|\| !` | Поддерживается, word forms предпочтительнее |
| Одинарные `&` и `\|` | Lexer tokens без рабочей bitwise/logical semantics |
| Группировка | `(expression)` | Явный приоритет |
| Вызов | `name(a, b)` | Обычная функция или builtin |
| Метод | `value.method(a)` | Метод struct или специальная runtime operation |
| Поле | `value.field`, `my.field` | Чтение поля |
| Индекс | `items[i]`, `text[i]` | List element или `Char`; вне диапазона fail-soft |

## Функции, labels и ошибки

| Форма | Канонический шаблон | Статус |
|---|---|---:|
| Функция без возврата | `fn log(Text value) { ... }` | Stable |
| Функция с возвратом | `fn add(Int a, Int b) returns Int { ... }` | Stable |
| Danger-функция | `danger fn load(Path path) returns Text { ... }` | Stable |
| Локальная функция | `local fn helper() { ... }` | Stable |
| Legacy return type | `fn add(...) Int { ... }` | Compatibility |
| Возврат | `return value`, `return` | Stable |
| Код ошибки | `return error MissingFile` | Stable, только в `danger fn` |
| Числовая метка | `label ErrorCode { Ok = 0 MissingFile = 1 }` | Stable; дискриминанты обязательны |
| Символический набор | `tag Status { Ready Busy }` | Stable; числовой контракт отсутствует |
| Danger recovery | `value = load(path) on error { ... }` | Stable |
| Danger call без результата | `save() on error { ... }` | Stable |
| Read-only borrow | `fn inspect(view Canvas frame)`, `inspect(view frame)` | Experimental |
| Mutable borrow | `fn paint(direct Canvas frame)`, `paint(direct frame)` | Experimental |
| Передача ownership | `fn consume(move Canvas frame)`, `consume(move frame)` | Experimental |
| Возврат ownership | `return move frame` | Experimental |

`ErrorCode` обязан существовать, а его первым вариантом должен быть `Ok`.

`view` передаёт ссылку без права изменять объект через этот параметр. `direct`
даёт исключительный изменяемый доступ. Оба borrow завершаются вместе с
синхронным вызовом и не могут пересекать границу `run`.

## Ветвления и циклы

| Форма | Пример | Статус |
|---|---|---:|
| Условие | `if ready { ... } else { ... }` | Stable |
| Цепочка | `else if condition { ... }` | Stable |
| Сопоставление | `when value { is 1 { ... } else { ... } }` | Stable |
| Условный цикл | `while condition { ... }` | Stable |
| Бесконечный цикл | `loop { ... }` | Stable |
| Канонический обход | `iterate items as item { ... }` | Stable |
| Альтернативный обход | `for item in items { ... }` | Stable |
| Управление циклом | `break`, `continue` | Stable |
| Пустой statement | `pass` | Stable |
| C-style loop | `for (i = 0; i < n; i++)` | Compatibility, semantic rejection |

## Struct и видимость

| Форма | Пример | Статус |
|---|---|---:|
| Struct | `struct Point { Float x Float y }` | Stable |
| Общий `label` | `label ExitCode { Success = 0 Failed = 1 }` | Stable numeric nominal type |
| Общий `tag` | `tag Status { Ready Busy }` | Stable symbolic nominal type |
| Скрытое поле | `hide Int secret` | Stable |
| Метод | `fn length() returns Float { ... }` внутри struct | Stable |
| Self access | `my.x` | Stable |
| Literal | `new Point p = {x = 1.0, y = 2.0}` | Stable |
| Field punning | `new Point p = {x, y}` | Stable |
| Локальный symbol | `local fn/struct` | Stable |
| Локальные nominal-типы | `local label Code { Ok = 0 }`, `local tag Status { Ready }` | Stable |

## Модули

| Форма | Статус | Примечание |
|---|---:|---|
| `import "./math.skd"` | Stable | Обрабатывается project pipeline |
| `math.add(1, 2)` | Stable | Квалификация по имени файла |
| `math.Point` | Stable | Квалифицированный тип |
| `math.SomeError` | Stable | Квалифицированный `ErrorCode` variant |
| `import module_name` | Future | Нет package/module resolver |
| `import "./x.skd" as x` | Future | Aliases отсутствуют |
| Re-export | Future | Отсутствует |

## Text, List и I/O builtins

| Функция | Сигнатура | Возвращает | Статус |
|---|---|---|---:|
| `len` | `len(Text\|List)` | `Int` | Stable |
| `contains` | `contains(Text, Text)` | `Bool` | Stable |
| `find` | `find(Text, Text)` | `Int` | Stable |
| `slice` | `slice(Text, Int, Int)` | `Text` | Stable |
| `concat` | `concat(Text, Text)` | `Text` | Stable |
| `args` | `args()` | `Text List` | Stable |
| `output` | `output(Int\|Float\|Bool\|Char\|Text)` | `Int` | Stable |
| `input` | `input(Text)` | `Text` | Stable |
| `read` | `read(Text\|Path)` | `Text` | Stable |
| `write` | `write(Text\|Path, Text)` | `Int` | Stable |
| `fs.list` | `fs.list(Text\|Path)` | `Text List` | Stable |
| `fs.join` | `fs.join(Text\|Path, Text)` | `Text` | Stable |
| `fs.is_dir` | `fs.is_dir(Text\|Path)` | `Bool` | Stable |

List operations:

```skadi
items.push(value)
value = items.pop() on error {
    output("empty list")
}
```

## Math constants и builtins

Constants без imports: `PI`, `TAU`, `E`, `EPSILON` (`Float`).

| Функция | Аргументы | Возвращает |
|---|---|---|
| `abs` | numeric | Сохраняет `Int`, иначе `Float` |
| `min`, `max` | numeric, numeric | Общий numeric type |
| `clamp` | numeric, numeric, numeric | Общий numeric type |
| `floor`, `ceil`, `round` | numeric | `Float` |
| `sqrt`, `root` | numeric; numeric, numeric | `Float` |
| `sin`, `cos` | `Angle` или legacy numeric radians | `Float` |
| `atan2` | numeric, numeric | `Angle` |
| `deg_to_rad` | numeric degrees | `Angle` |
| `rad_to_deg` | `Angle` | `Float` |
| `dot` | два одинаковых vector type | `Float` |
| `length`, `length_sq` | vector | `Float` |
| `normalize` | vector | Тот же vector type |
| `distance`, `distance_sq` | два одинаковых vector type | `Float` |
| `cross` | `Vec3`, `Vec3` | `Vec3` |

## Time, Memory, Task и Channel

| Форма | Смысл | Статус |
|---|---|---:|
| `now()` | Monotonic timestamp `Time` | Experimental |
| `elapsed(start)` | Прошедший `Duration` | Experimental |
| `sleep(duration)` | Blocking sleep | Experimental |
| `delay(duration)` | Alias-подобная blocking delay | Experimental |
| `Memory arena = memory(4kb)` | Fixed-capacity region | Experimental |
| `memory(4kb, allow grow)` | Segmented growing region | Experimental |
| `memory(..., allow drop)` | Явный policy marker без implicit reclamation | Experimental |
| `memory.child(1kb)` | Fixed child активного parent-region | Experimental |
| `memory.static(4kb)` | Root-only static buffer | Experimental |
| `place in arena { ... } on error { ... }` | Region placement/recovery | Experimental |
| `arena.clear()` | Очистка region | Experimental |
| `Task work = run worker()` | Запуск void task | Experimental |
| `Task(Int) work = run compute()` | Запуск task с результатом | Experimental |
| `result = wait work` | Join и получение результата | Experimental |
| `result = wait work for 250ms on error { ... }` | Timed wait; success поглощает handle, timeout сохраняет его в handler | Experimental |
| `stop work` | Cooperative stop request; будит blocking Channel operation | Experimental |
| `stopping` | Флаг внутри task function | Experimental |
| `Channel(Int) jobs = channel(8)` | Bounded channel owner | Experimental |
| `jobs.send(value)` | Blocking send | Experimental |
| `value = jobs.receive()` | Blocking receive | Experimental |
| `jobs.send(value) on error { ... }` | Обработка `close` или cancellation | Experimental |
| `value = jobs.receive() on error { ... }` | Обработка drain/cancellation; `stopping` различает stop | Experimental |
| `jobs.try_send(value)` | Неблокирующая попытка отправки, результат `Bool` | Experimental |
| `jobs.close()` | Owner закрывает поток; буфер дочитывается | Experimental |
| `jobs.send_for(value, 250ms) on error { ... }` | Blocking send с deadline | Experimental |
| `value = jobs.receive_for(250ms) on error { ... }` | Blocking receive с deadline | Experimental |
| `timed_out` | Причина timed Channel/Task handler; контекстный identifier | Experimental |

## Specialized arithmetic

| Тип | Разрешено |
|---|---|
| `Time` | `Time - Time -> Duration`, `Time +/- Duration -> Time`, comparisons |
| `Duration` | `Duration +/- Duration`, comparisons |
| `ByteSize` | `ByteSize +/- ByteSize`, comparisons |
| `Angle` | `+/- Angle`, scalar `*`/`/`, `Angle / Angle -> Float`, comparisons |
| Vector | Same-dimension `+/-`, unary `-`, scalar `*`/`/`, component access |

Смешивание specialized type с голым `Int/Float` запрещено, кроме явно
перечисленных scalar operations.

## Canvas v0 (`v1.2`, experimental)

| Форма | Назначение |
|---|---|
| `color(r, g, b, a)` | Создать `Color` из RGBA `0..255` |
| `color_hex("#rrggbb", a)` | Создать `Color` из hex и alpha |
| `Color.terminal_blue` | Мягкая 16-цветная терминальная палитра |
| `rect(x, y, width, height)` | Создать `Rect` |
| `canvas(width, height)` | Создать software `Canvas` |
| `canvas.clear(color)` | Очистить кадр |
| `canvas.pixel(position, color)` | Пиксель |
| `canvas.line(from, to, color)` | Линия |
| `canvas.rect(area, color)` | Рамка прямоугольника |
| `canvas.fill_rect(area, color)` | Заполненный прямоугольник |
| `canvas.circle(center, radius, color)` | Окружность |
| `canvas.fill_circle(center, radius, color)` | Заполненный круг |
| `canvas.checksum()` | Детерминированная проверка кадра |
| `windows.open(title, width, height)` | Создать Win32 `Window` |
| `window.present(direct canvas)` | Показать Canvas с явным borrow |
| `window.is_open()` / `window.close()` | Lifecycle окна |

`Color` и `Rect` — value types. `Canvas` и `Window` — линейные ресурсы: они не
копируются, временно передаются через `view`/`direct`, а ownership передаётся
через `move`. Полный текущий контракт:
[Canvas и Visual Core](canvas.md).

## Не является рабочим синтаксисом

| Идея | Статус |
|---|---:|
| `Interrupt tick = interrupts.periodic(10ms)` | Host MVP: typed periodic source |
| `on interrupt tick { channel.try_send(1) }` | Host MVP: строгий interrupt context |
| Отдельный `on error { ... }` | Reserved: recovery должен быть связан с danger-вызовом |
| `fixed`, `const` | Lexer-only reservations |
| Одинарные `&`, `\|` и общий `:` | Lexer-only tokens без текущей semantic формы |
| `allow grow`, `allow drop` вне `memory(...)` | Не является общей языковой формой |
| Channel `select` и `try_receive` | Future |
| async/await, futures, task groups | Future |
| `Matrix2D`, Canvas text/images/events и non-Windows Window | Future |
| Generics, decorators, operator overloading | Conscious non-goals текущего языка |

Подробности: [полная справка](language-reference.md) и
[точный статус синтаксиса](syntax-status.md).
