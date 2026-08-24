# Базовые типы, литералы и выражения

## Исходный файл

Skadi использует расширение `.skd`. Перевод строки завершает большинство
statements; обязательные `;` не нужны.

```skadi
// Строчный комментарий
new Text title = "Skadi"

/*
Блочный комментарий
*/
output(title)
```

## Объявления

```skadi
new count = 10
new Int limit = 100
new Text name = "Ada"
new Int List values = [2, 4, 8]
```

Вывод типа предназначен для скалярных значений. Для `List`, struct, vectors,
`Task` и `Channel` указывайте тип явно.

```skadi
count = count + 1
count++
```

`+=`, `-=`, `*=`, `/=` читаются, но formatter переводит их в явное
присваивание. Неизменяемое значение объявляется через `constant`:

```skadi
constant Int max_retries = 3
```

Слова `fixed` и `const` не являются ключевыми и могут использоваться как
обычные идентификаторы. Они не являются альтернативами `constant`.

## Типы

Основные stable-типы:

| Семейство | Типы |
|---|---|
| Целые | `Int`, `i8/i16/i32/i64`, `u8/u16/u32/u64` |
| Вещественные | `Float`, `f32`, `f64` |
| Логические | `Bool` (`bool` — alias) |
| Символ | `Char` (`char` — alias) |
| Текст | `Text`, `Path` |
| Коллекция | `ElementType List` |
| Пользовательские | Имена `struct` |

Experimental specialized types: `Time`, `Duration`, `ByteSize`, `Angle`,
`Vec2/Vec3/Vec4`, `Memory`, `Task(T)` и `Channel(T)`.

`Int -> Float` является поддержанным widening. Между fixed-width integer
переменными implicit casts нет: используйте `as_i8..as_i64` или
`as_u8..as_u64` с `on error`. Эти преобразования принимают и вещественные
значения, но только конечные, целые и помещающиеся в target. Для `f32` есть
проверяемый `as_f32(...) on error`, а `as_f64(...)` выполняет безопасное
расширение. Подходящий целый литерал можно записывать сразу в нужный
fixed-width тип.

`Int` имеет ширину target profile и настраивается в `skadi.toml` через
`[numeric] int = "target|i8|i16|i32|i64"`; desktop default — `i32`. Для ABI,
FFI, протоколов и битовых операций используйте fixed-width типы. `Float` и
`f32` являются 32-битными, `f64` всегда задаётся явно и сохраняет double-точность
литералов и математики. Двоичные, восьмеричные и
шестнадцатеричные литералы записываются как `0b1010`, `0o755`, `0xff`; внутри
числа разрешён `_`.

Обычная целочисленная арифметика проверяет переполнение и деление на ноль;
runtime-ошибка имеет код `SC-RT-350`. Намеренное переполнение выражается явно
через `wrapping_add/sub/mul/neg` и доступно только fixed-width типам.

## Литералы

```skadi
new Int count = 42
new Float ratio = 0.75
new Bool ready = true
new Char marker = '\n'
new Text message = "ready"
new Int List ids = [3, 5, 8]
new Point point = {x = 1.0, y = 2.0}

new Duration timeout = 250ms
new ByteSize budget = 4kb
new Angle quarter = 90deg
```

`Char` ограничен ASCII. Строковый `Text` использует byte-oriented runtime:
индексация возвращает `Char`, но не Unicode code point.

## Операторы

```skadi
new Int total = (a + b) * 2
new Int page = total div 10
new Int rest = total mod 10
new Bool valid = total > 0 and not disabled
```

Доступны `+ - * / % ^`, `div`, `mod`, сравнения, `and/or/xor/not`.
Symbolic `&&`, `||`, `!` также работают, но word forms лучше соответствуют
стилю Skadi.

## Доступ и индексация

```skadi
new Int second = values[1]
new Char first = message[0]
new Float x = point.x
```

Индекс вне диапазона возвращает fail-soft default: для `Text` это `'\0'`.
Typed error на индексации пока отсутствует.

Сводная таблица всех форм: [быстрая справка](language-quick-reference.md).
