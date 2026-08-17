# Статус синтаксиса Skadi

Дата: 2026-07-30
Назначение: единый точный срез того, какой синтаксис действительно работает в этом репозитории сейчас.

## Уровни статуса

- `Stable` - реализовано, покрыто тестами, ожидается как рабочая часть языка.
- `Partial` - реализовано с явными ограничениями или переходным поведением.
- `Experimental` - реализуется как текущий `v1.2` track, но ещё не является stable runtime surface.
- `Compatibility` - принимается для старого кода, но formatter или style policy
  направляет к канонической форме.
- `Reserved` - распознаётся только частью frontend и не является компилируемой
  пользовательской поверхностью.
- `Planned` - не входит в текущую рабочую поверхность этого репозитория.

## Базовые конструкции

- `new x = scalar_expr` - `Stable`
  - C-тип выводится для scalar values, включая `Text`, `Duration`, `ByteSize` и `Angle`;
  - `List`, структуры, векторы, Task, Channel и неизвестный composite type требуют явной аннотации (`SC-SEM-020`).
- `new Type x = expr` - `Stable`
- `new ElemType List x = [...]` - `Stable`
- `x = expr` - `Stable`
- `x.field = expr` - `Stable`
- `i++` / `i--` - `Stable`
- `+=`, `-=`, `*=`, `/=` - `Stable input / canonicalized`
  - statement parser безопасно раскрывает форму через соответствующую binary operation;
  - semantic использует обычные type rules, formatter пишет явную форму `x = x + value`;
  - compound operator нельзя использовать вместо `=` в declaration initializer.
- `return expr` - `Stable`
- `return` - `Stable`
- `return error Code` - `Stable`
- `pass` - `Stable`
- выражение как statement, включая builtin-вызовы вроде `output("hello")` - `Stable`
- `direct Type name` / `direct value` - `Implemented / bounded mutable borrow`
- `view Type name` / `view value` - `Implemented / bounded read-only borrow`
- `move Type name` / `move value` / `return move value` -
  `Implemented / bounded ownership transfer`
  - применяется к `Canvas`, `Window`, `Interrupt` и owning `Channel`;
  - старое имя после передачи недоступно;
  - partial move после ветвления и move из повторяющегося loop диагностируются.
- `constant direct` - `Removed`; parser указывает использовать `view`
- `fixed` / `const` - `Reserved / Not implemented`
  - lexer распознаёт эти слова, но текущий statement parser намеренно не принимает формы;
  - они не должны использоваться в пользовательском коде.

## Функции

- `fn name(...) { ... }` - `Stable`
- `danger fn name(...) { ... }` - `Stable`
- `local fn name(...) { ... }` - `Stable`
- типизированные параметры - `Stable`
- канонический типизированный возврат `fn name(...) returns Type` - `Stable`
- legacy-возврат `fn name(...) Type` - `Partial`
  - scalar, specialized и struct-формы принимаются только для совместимости с предупреждением;
  - formatter всегда переводит их в `fn name(...) returns Type`;
  - новый код должен использовать `returns`.
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
- legacy `for (init; cond; update)` - `Partial`
  - parser и formatter сохраняют форму для чтения старых исходников;
  - semantic выдаёт `SC-SEM-040`: backend не исполняет эту форму;
  - используйте `iterate collection as item` или `for item in collection`.
- `when / is / else` - `Stable`

## Структуры и методы

- `struct Name { ... }` - `Stable`
- `local struct Name { ... }` - `Stable`
- `label Name { A = 0 B = 1 }` и local-вариант - `Stable`
  - каждый числовой дискриминант обязателен;
  - `ErrorCode` начинается с `Ok = 0`.
- `tag Name { A B }` и local-вариант - `Stable`
  - символический nominal type без пользовательского числового контракта.
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
- публичные `fn`, `struct`, `label` и `tag` импортируются только напрямую - `Stable`
- `local fn/struct/label/tag` не экспортируются из файла - `Stable`
- коллизии публичных символов диагностируются как `SC-MOD-002` - `Stable`
- нарушение direct-import-only видимости диагностируется как `SC-MOD-003` - `Stable`
- квалификация `module.symbol`, где `module` - имя файла без `.skd`, работает для функций, типов структур и вариантов `ErrorCode` - `Stable`
- `import "./x.skd" as alias` - `Stable`; alias локален текущему файлу
- `import module_name` - `Planned`

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
  - значения доступны через индексирование `Text` и ASCII literals;
  - поддержаны escapes `'\n'`, `'\r'`, `'\t'`, `'\0'`, `'\''`, `'\\'`;
  - Unicode, empty и multi-character literals отклоняются с `SC-PARSE-221`.
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

- `Interrupt tick = interrupts.periodic(Duration)` - `Host MVP`
- `on interrupt tick { ... }` - `Host MVP`
  - регистрация разрешена владельцу на top level;
  - handler допускает конечные scalar-вычисления и `Channel.try_send`;
  - blocking, allocation, I/O, task/resource management и обычные вызовы запрещены;
  - hardware IRQ backends остаются planned.
- time/duration systems MVP - `Experimental / Runtime MVP`
  - nominal types `Time` и `Duration` проходят parser/semantic/C codegen;
  - integer literals `ms`, `s`, `min` проверяются на overflow;
  - `now`, `elapsed`, `sleep`, `delay` работают через Win32/POSIX monotonic runtime;
  - разрешена только явная time/duration арифметика без смешивания с `Int/Float`;
  - `Time` и `Duration` value-safe для struct/List/Task/Channel;
  - wall-clock, `Timer`, fractional literals и embedded backend отложены;
  - полный контракт: [Время и длительности](time-duration.md).
- byte-size systems MVP - `Experimental / Runtime MVP`
  - nominal type `ByteSize` проходит parser/semantic/C codegen;
  - integer literals `b`, `kb`, `mb`, `gb` используют бинарные множители и проверяются на overflow;
  - `ByteSize +/- ByteSize` и сравнения одинаковых типов работают без смешивания с `Int/Float`;
  - `memory(...)` принимает `ByteSize` expression, а non-positive capacity отклоняется runtime;
  - `ByteSize` value-safe для struct/List/Task/Channel;
  - allocator policies, fractional units и dimensional algebra отложены;
  - полный контракт: [Размеры памяти](byte-size.md).
- angle math MVP - `Experimental / Runtime MVP`
  - nominal `Angle` хранится как `f64` radians;
  - integer/fractional literals `deg` и `rad` проверяются на finite value;
  - angle arithmetic, comparisons, limited scalar operations и value-safe boundaries реализованы;
  - `deg_to_rad` и `atan2` возвращают `Angle`, `rad_to_deg` принимает `Angle`;
  - `sin/cos` принимают `Angle`, сохраняя numeric raw-radians compatibility для `v1.1`;
  - dimensional algebra, normalization и fixed-point embedded representation отложены;
  - полный контракт: [Углы](angle.md).
- vector math MVP - `Experimental / Runtime MVP`
  - `Vec2`, `Vec3`, `Vec4` имеют `f64` components и точную structural-literal форму;
  - component access/assignment, same-dimension `+/-`, unary minus и scalar `*//` реализованы;
  - `dot`, `length`, `length_sq`, `normalize`, `distance`, `distance_sq` работают для всех трёх типов;
  - `cross` принимает только два `Vec3`, zero normalization возвращает zero vector;
  - векторы value-safe для struct/List/Task/Channel;
  - matrices, generic/SIMD vectors, swizzling и operator overloading отложены;
  - полный контракт: [Векторы](vectors.md).
- memory model MVP surface - `Experimental / Partial`
  - frontend принимает `Memory name = memory(size[, policies])`, `memory.child`,
    `memory.static`, `place in memory { ... } on error { ... }` и `memory.clear()`;
  - semantic layer проверяет базовые escape / use-after-clear правила только для dynamic payload (`Text`, `List`, и struct-значений с такими полями);
  - `Memory` считается capability/resource handle, а не обычным storable value type;
  - `allow grow` использует segmented chunks без перемещения старых allocations;
  - `allow drop` хранится как policy marker и не удаляет живые значения автоматически;
  - child-region получает fixed storage из активного parent, static-region
    требует literal capacity и разрешён только на program root;
  - C backend доводит весь bounded surface до `Skadi -> C -> native`.
- task/channel systems MVP - `Experimental / Runtime MVP`
  - parser принимает `Task`, `Task(T)`, `run worker(...)`, `wait task`, timed
    `wait task for Duration on error`, `stop task`, `stopping`, `Channel(T)`,
    `channel(N)`, blocking и timed Channel operations;
  - semantic layer проверяет task handle lifecycle, запрет `Task` как обычного value-type, task-context для `stopping` и value-safe channel messages;
  - игнорирование результата `run worker()` является hard error;
  - semantic pass требует `wait` на всех путях и проверяет task-safe boundary;
  - `Task = run void_fn(...)`, `Task(T) = run fn(...)`, `stop`, `stopping` и `wait` работают через Win32/pthread backend;
  - `stop` является кооперативным запросом и не отменяет обязательный `wait`;
  - bounded `Channel(T)` работает через blocking FIFO `send/receive` на Win32/pthread;
  - mutable `List`, Memory/capability и region-owned значения не являются value-safe сообщениями;
  - owner declaration внутри loop и `place in` запрещён ради deterministic cleanup;
  - owner может вызвать `close()`, после чего receiver дренирует очередь и
    обрабатывает исчерпание через `on error`;
  - `try_send` возвращает `Bool` и не блокирует producer;
  - `stop` пробуждает task, заблокированную в `send/receive`; cancellation
    обрабатывается через `on error` и не закрывает Channel;
  - `send_for(value, Duration)` и `receive_for(Duration)` требуют `on error`;
  - `wait task for Duration on error { ... }` выполняет path-sensitive timed
    join: success поглощает handle, timeout сохраняет его живым в handler;
  - `timed_out` доступен только в handler timed Channel/Task и не является lexer keyword;
  - `try_receive` и `select` отложены.
  - практические шаблоны и платформенный статус описаны в
    [руководстве по многопоточности](concurrency.md).
- Canvas v0 - `Experimental / Runtime MVP`
  - `Color` и `Rect` являются value-safe типами;
  - `color`, `color_hex`, мягкие цветовые алиасы и 16 `Color.terminal_*` констант реализованы;
  - `Canvas` является linear resource с software RGBA framebuffer;
  - `clear`, `pixel`, `line`, `rect`, `fill_rect`, `circle`, `fill_circle` и `checksum` проходят semantic/C runtime;
  - `Window` является отдельным linear resource, а `present` требует `direct Canvas`;
  - первый Window backend реализован для Win32; headless Canvas остаётся переносимым;
  - events, text/images, transforms, `Matrix2D` и остальные оконные backend отложены;
  - полный контракт: [Canvas и Visual Core](canvas.md).
- formatter coverage - `Partial`
  - ориентирован на текущий рабочий слой `v1.1` и экспериментальные формы `v1.2`, где это безопасно;
  - уже пригоден для повседневной работы, но продолжает развиваться вместе с синтаксисом.

## Сознательно отложенное

- module-name imports и aliases поверх стабильного path-import контракта
- расширение Canvas: events, text/images, transforms и дополнительные backends
- systems additions track
- более строгая модель ошибок индексации
- async/background execution внутри TUI

## Примечание

Этот файл фиксирует текущий реализованный контракт, а не вечную финальную форму языка.
Для первого знакомства удобнее начинать с [Начало работы](getting-started.md).
Полная компактная матрица находится в
[быстрой справке по языку](language-quick-reference.md).
