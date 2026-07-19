# Статус синтаксиса Skadi

Дата: 2026-07-19
Назначение: единый точный срез того, какой синтаксис действительно работает в этом репозитории сейчас.

## Уровни статуса

- `Stable` - реализовано, покрыто тестами, ожидается как рабочая часть языка.
- `Partial` - реализовано с явными ограничениями или переходным поведением.
- `Experimental` - реализуется как текущий `v1.2` track, но ещё не является stable runtime surface.
- `Planned` - не входит в текущую рабочую поверхность этого репозитория.

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
- `local fn name(...) { ... }` - `Stable`
- типизированные параметры - `Stable`
- канонический типизированный возврат `fn name(...) returns Type` - `Stable`
- legacy-возврат `fn name(...) Type` - `Partial`
  - scalar-формы пока принимаются с предупреждением;
  - возврат структуры требует явного `returns`.
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
- `local struct Name { ... }` и `local label Name { ... }` - `Stable`
- поля структуры - `Stable`
- скрытые поля `hide Type field` - `Stable`
  - доступны только методам той же структуры;
- методы внутри структуры - `Stable`
- `my.field` внутри методов - `Stable`
- доступ `obj.field` - `Stable`
- вызовы `obj.method(...)` - `Stable`
- struct literals `{field = value, ...}` - `Stable`
- field punning `{value, status}` - `Stable`
- списки структур и вызовы методов на итерируемых элементах - `Stable`

## Импорты, модули и видимость

- path-import `import "./relative/path.skd"` - `Stable`
- циклические и отсутствующие импорты диагностируются как `SC-MOD-001` - `Stable`
- публичные `fn`, `struct` и `label` импортируются только напрямую - `Stable`
- `local fn/struct/label` не экспортируются из файла - `Stable`
- коллизии публичных символов диагностируются как `SC-MOD-002` - `Stable`
- нарушение direct-import-only видимости диагностируется как `SC-MOD-003` - `Stable`
- квалификация `module.symbol`, где `module` - имя файла без `.skd`, работает для функций, типов структур и вариантов `ErrorCode` - `Stable`
- `import module_name` и `import "./x.skd" as alias` - `Planned`

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
  - семантика выполнения ещё не считается завершённой stable частью языка.
- time/duration systems MVP - `Experimental / Runtime MVP`
  - nominal types `Time` и `Duration` проходят parser/semantic/C codegen;
  - integer literals `ms`, `s`, `min` проверяются на overflow;
  - `now`, `elapsed`, `sleep`, `delay` работают через Win32/POSIX monotonic runtime;
  - разрешена только явная time/duration арифметика без смешивания с `Int/Float`;
  - `Time` и `Duration` value-safe для struct/List/Task/Channel;
  - wall-clock, `Timer`, fractional literals и embedded backend отложены;
  - полный контракт: [Время и длительности](time-duration.md).
- memory model MVP surface - `Experimental / Partial`
  - frontend принимает `Memory name = memory(size)`, `place in memory { ... } on error { ... }` и `memory.clear()`;
  - semantic layer проверяет базовые escape / use-after-clear правила только для dynamic payload (`Text`, `List`, и struct-значений с такими полями);
  - `Memory` считается capability/resource handle, а не обычным storable value type;
  - C backend уже lower'ит strict MVP surface в fixed-capacity region runtime и доводит её до `Skadi -> C -> native`;
  - `allow grow`, `allow drop`, `memory.child`, `memory.static` остаются design-level future surface.
- task/channel systems MVP - `Experimental / Runtime MVP`
  - parser принимает `Task`, `Task(T)`, `run worker(...)`, `wait task`, `stop task`, `stopping`, `Channel(T)`, `channel(N)`, `channel.send(value)` и `channel.receive()`;
  - semantic layer проверяет task handle lifecycle, запрет `Task` как обычного value-type, task-context для `stopping` и value-safe channel messages;
  - игнорирование результата `run worker()` является hard error;
  - semantic pass требует `wait` на всех путях и проверяет task-safe boundary;
  - `Task = run void_fn(...)`, `Task(T) = run fn(...)`, `stop`, `stopping` и `wait` работают через Win32/pthread backend;
  - `stop` является кооперативным запросом и не отменяет обязательный `wait`;
  - bounded `Channel(T)` работает через blocking FIFO `send/receive` на Win32/pthread;
  - mutable `List`, Memory/capability и region-owned значения не являются value-safe сообщениями;
  - owner declaration внутри loop и `place in` запрещён ради deterministic cleanup;
  - `close`, timeout, `select` и отмена блокирующей channel operation отложены.
  - практические шаблоны и платформенный статус описаны в
    [руководстве по многопоточности](concurrency.md).
- formatter coverage - `Partial`
  - ориентирован на текущий рабочий слой `v1.1` и экспериментальные формы `v1.2`, где это безопасно;
  - уже пригоден для повседневной работы, но продолжает развиваться вместе с синтаксисом.

## Сознательно отложенное

- module-name imports и aliases поверх стабильного path-import контракта
- visual core / canvas
- systems additions track
- более строгая модель ошибок индексации
- async/background execution внутри TUI

## Примечание

Этот файл фиксирует текущий реализованный контракт, а не вечную финальную форму языка.
Для первого знакомства удобнее начинать с [Начало работы](getting-started.md).
