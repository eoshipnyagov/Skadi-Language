# Scadi: Документация Языка (Текущее Состояние v0.1)

Дата актуальности: 2026-05-22  
Статус: практическая документация по **реально реализованному** в текущем прототипе.

## 1. Что такое Scadi сейчас

Scadi в этом репозитории сейчас реализован как:
1. лексер,
2. парсер,
3. семантический анализатор,
4. транспилятор в C.

Поддерживается рабочий end-to-end пайплайн:
`Skadi source -> tokens -> AST -> semantic checks -> C code`.

## 2. Базовый синтаксис

- Разделение инструкций: в текущем состоянии ориентир на “одна инструкция на строку”.
- Блоки: `{ ... }`.
- Комментарии: `// ...`, `/* ... */`.
- Объявление переменной:
  - `new x = 1`
  - `new Int x = 1`
  - `new i32 List xs = [1, 2, 3]`
- Присваивание:
  - `x = 2`
- Вызов функции:
  - `y = add(x, 2)`
- Опасный вызов с обработкой:
  - `x = parse(x) on error { x = 0 }`

## 3. Поддерживаемые конструкции языка

### 3.1 Декларации и типы

Поддерживаются:
- `new` с выводом типа,
- `new` с явным типом (`Int`, `Float`, `i32`, `u8`, `bool`, `char`, `Text`, `... List`),
- литералы:
  - целые,
  - float,
  - bool,
  - строковые.

### 3.2 Функции

Поддерживаются:
- обычные `fn`,
- `danger fn`,
- typed-сигнатуры параметров и возвращаемого значения,
- `return`,
- `return error <Variant>` внутри `danger fn`.

### 3.3 Управление потоком

Поддерживаются:
- `if / else`,
- `while`,
- `loop`,
- `when / is / else`,
- `for item in collection`,
- алиас цикла: `iterate collection as item`.

### 3.4 Коллекции List

Поддерживаются:
- типизированные литералы списков: `new i32 List xs = [1, 2, 3]`,
- `xs.push(v)`,
- `x = xs.pop() on error { ... }`,
- `len(xs)`,
- `xs[i]`.

Текущая runtime-семантика индексации:
- out-of-range для `List` в C-lowering даёт безопасное значение по умолчанию `0`
  (fail-soft поведение прототипа).

### 3.5 Текст Text

Поддерживаются:
- `new Text t = "hello"`,
- `len(t)`,
- `t[i]`,
- `contains(t, "sub") -> bool`,
- `find(t, "sub") -> Int` (индекс или `-1`),
- `slice(t, start, end) -> Text`.

Текущая runtime-семантика:
- `slice` нормализует границы (`start/end clamp`),
- `Text` операции byte-oriented (UTF-8 байты, не графемы),
- `t[i]` при выходе за границы в C-lowering возвращает `'\0'` (fail-soft).

### 3.6 Ошибки и `on error`

Поддерживается:
- `on error` для:
  - вызовов `danger fn`,
  - `List.pop()`.

Ограничение:
- `on error` для произвольных выражений/операций пока не реализован как универсальный механизм.

### 3.7 Label / Struct / On-block

Поддерживаются:
- `label` (включая `label ErrorCode`),
- `struct` (parse-level/placeholder в backend),
- `on interrupt ... { ... }` (parse/semantic-level).

## 4. Семантические правила (реализованные)

- проверка `use-before-definition`,
- запрет повторного объявления в одной области,
- запрет самоссылки в инициализации,
- проверка совместимости типов в присваиваниях/инициализации,
- проверка сигнатур и аргументов функций,
- проверка контекста `on error` (только `danger fn`),
- правила для `ErrorCode`:
  - если label присутствует, первый вариант должен быть `Ok`,
  - `return error X` проверяет существование `X`,
- проверка типов `List`/`Text` builtins (`len/contains/find/slice`),
- вывод типа переменной цикла `for/iterate` из коллекции.

## 5. Транспиляция в C (текущее поведение)

Генерируется:
- C-файл с `main`,
- runtime helper-ы для `List`,
- runtime helper-ы для `Text`,
- lowering control flow (`if`, `while`, `when`, `for`),
- lowering `danger fn` в сигнатуру с `out`-параметром.

Примеры lowering:
- `len(t)` -> `strlen(t)`,
- `find(t, s)` -> `sk_text_find(t, s)`,
- `slice(t, a, b)` -> `sk_text_slice(t, a, b)`,
- `t[i]` -> `sk_text_char_at(t, i)`,
- `xs[i]` -> `sk_list_<type>_get(&xs, i)`.

## 6. Что ещё не финализировано

- Полная модель памяти “чанков” из языкового дизайна пока не реализована.
- `on event`, `Link`, `run/wait` runtime-семантика ещё не реализованы.
- `struct` lowering в C пока TODO.
- Индексация сейчас fail-soft; переход к строгому danger/on-error контракту — отдельное design-решение.

## 7. Минимальный пример

```skadi
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn safe_div(Int a, Int b) Int {
    if b == 0 {
        return error ZeroDivision
    } else {
        return a div b
    }
}

new Text t = "weather station"
new bool has = contains(t, "station")
new Int idx = find(t, "ther")
new Text part = slice(t, 3, 7)
```
