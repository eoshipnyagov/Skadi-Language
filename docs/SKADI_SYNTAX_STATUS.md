# Статус синтаксиса Skadi

Дата: 2026-06-05  
Назначение: единый точный срез того, какой синтаксис действительно работает в этом репозитории сейчас.

## Уровни статуса

- `Stable` - реализовано, покрыто тестами, ожидается как рабочая часть языка.
- `Partial` - реализовано с явными ограничениями или переходным поведением.
- `Planned` - не входит в рабочую поверхность `v1.1` в этом репозитории.

## Базовые конструкции

- `new x = expr` - `Stable`
- `new Type x = expr` - `Stable`
- `new ElemType List x = [...]` - `Stable`
- `x = expr` - `Stable`
- `x.field = expr` - `Stable`
- `i++` / `i--` - `Stable`
- `return expr` - `Stable`
- `return` - `Stable`
- `return error Code` - `Stable`
- `pass` - `Stable`
- выражение как statement, включая builtin-вызовы вроде `output("hello")` - `Stable`

## Функции

- `fn name(...) { ... }` - `Stable`
- `danger fn name(...) { ... }` - `Stable`
- типизированные параметры - `Stable`
- типизированный возврат - `Stable`
- вызовы функций внутри выражений - `Stable`
- проверка количества и типов аргументов - `Stable`

## Поток ошибок

- `x = danger_call(...) on error { ... }` - `Stable`
- `danger_call(...) on error { ... }` - `Stable`
- `on error` только на danger-вызовах - `Stable`
- контракт `label ErrorCode` - `Stable`
  - первый вариант должен быть `Ok`
  - `return error X` требует существующего варианта `ErrorCode`

## Управляющие конструкции

- `if / else if / else` - `Stable`
- `while` - `Stable`
- `loop` - `Stable`
- `break` / `continue` - `Stable`
- `for item in collection` - `Stable`
- `iterate collection as item` - `Stable`
  - предпочтительная витринная форма записи
- legacy `for (init; cond; update)` - `Stable`
  - поддерживается для совместимости, но не считается предпочтительным стилем
- `when / is / else` - `Stable`

## Структуры и методы

- `struct Name { ... }` - `Stable`
- поля структуры - `Stable`
- методы внутри структуры - `Stable`
- `my.field` внутри методов - `Stable`
- доступ `obj.field` - `Stable`
- вызовы `obj.method(...)` - `Stable`
- struct literals `{field = value, ...}` - `Stable`
- field punning `{value, status}` - `Stable`
- списки структур и вызовы методов на итерируемых элементах - `Stable`

## Builtins: Text / List / Filesystem / I/O

- `len` - `Stable`
- `contains` - `Stable`
- `find` - `Stable`
- `slice` - `Stable`
- `concat` - `Stable`
- `fs.list` - `Stable`
- `fs.is_dir` - `Stable`
- `fs.join` - `Stable`
- `args` - `Stable`
- `output` - `Stable`
- `input` - `Stable`
- `read` - `Stable`
- `write` - `Stable`

## Math core (`v1.1`)

- константы `PI`, `TAU`, `E`, `EPSILON` - `Stable`
- `abs`, `min`, `max`, `clamp` - `Stable`
- `floor`, `ceil`, `round` - `Stable`
- `sin`, `cos`, `atan2`, `sqrt`, `root` - `Stable`
- `deg_to_rad`, `rad_to_deg` - `Stable`

## Типы

- `Int` - `Stable`
- `Float` - `Stable`
- `Bool` / `bool` - `Stable`
- `Char` / `char` - `Stable`
- `Text` - `Stable`
- `Path` - `Stable`
- контейнеры `List` - `Stable`
- fixed-width numeric families:

  - `i8`, `i16`, `i32`, `i64`
  - `u8`, `u16`, `u32`, `u64`
  - `f32`, `f64`
  - `Stable`

## Контракт индексации

- `xs[i]` для `List` - `Stable`
- `t[i]` для `Text` - `Stable`
- индекс списка вне диапазона возвращает fail-soft default value - `Stable`
- индекс текста вне диапазона возвращает `'\0'` - `Stable`
- `on error` на индексации - `Planned`

## Стиль и канонические формы

- `iterate ... as ...` предпочтительнее `for ... in ...` - `Stable warning policy`
- `Bool` предпочтительнее `bool` - `Stable warning policy`
- `Char` предпочтительнее `char` - `Stable warning policy`

## Частично реализованное / переходное

- `on interrupt ... { ... }` - `Partial`
  - parse-level поддержка уже есть;
  - семантика выполнения ещё не считается завершённой частью `v1.1`.
- formatter coverage - `Partial`
  - ориентирован на текущий рабочий слой `v1.1`;
  - уже пригоден для повседневной работы, но продолжает развиваться вместе с синтаксисом.

## Сознательно отложенное

- imports / modules
- task / channel concurrency model
- memory model features
- visual core / canvas
- systems additions track
- более строгая модель ошибок индексации
- async/background execution внутри TUI

## Примечание

Этот файл фиксирует текущий реализованный контракт, а не вечную финальную форму языка.
Для первого знакомства удобнее начинать с [Начало работы](getting-started.md).
