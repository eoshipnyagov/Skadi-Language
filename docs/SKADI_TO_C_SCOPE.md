# Границы компиляции Skadi -> C

## Статус

Дата сверки: 2026-07-29

Stable base `v1.1` и experimental systems slices `v1.2` проходят общий pipeline:

```text
Skadi -> lexer -> parser -> semantic -> C codegen -> host C compiler -> binary
```

Эта страница описывает фактический backend, а не будущую спецификацию языка.

## Core lowering

Текущий C backend поддерживает:

- top-level statements и функции `fn` / `danger fn`;
- typed params и canonical `returns`;
- scalar и fixed-width types;
- `new`, assignment, `i++`, `i--`;
- arithmetic, comparison, logical operators и `^ -> pow`;
- `if/else`, `while`, `loop`, `for in`, `iterate as`;
- `when/is/else`, `break`, `continue`, `pass`;
- `label ErrorCode`, `return error`, `on error`;
- struct declarations, literals, fields, `my.field` и methods;
- `local`/`hide` после CLI module preprocessing;
- relative path imports и `module.symbol` после CLI merge pipeline.

## Type mapping

| Skadi | C |
|---|---|
| `Int`, `i64` | `int64_t` |
| `i8/i16/i32` | `int8_t/int16_t/int32_t` |
| `u8/u16/u32/u64` | `uint8_t/uint16_t/uint32_t/uint64_t` |
| `Float`, `f64` | `double` |
| `f32` | `float` |
| `Bool` | `bool` |
| `Char` | `char` |
| `Text`, `Path` | managed `char*` runtime representation |
| `Time`, `Duration` | nominal Skadi types lowered to `int64_t` nanoseconds |
| `ByteSize` | nominal Skadi type lowered to signed `int64_t` bytes |
| `Angle` | nominal Skadi type lowered to `double` radians |
| `Vec2`, `Vec3`, `Vec4` | value structs из 2/3/4 `double` components |
| user struct | generated C `typedef struct` |

Nominal semantic rules сохраняются до codegen: совпадающее C representation не
разрешает неявно смешивать `Time/Duration/ByteSize/Angle` с numeric types.

## Collections, text and I/O

Runtime helpers реализуют:

- typed mutable `List` families, iteration, index, `push`, `pop`;
- `Text` length/index/search/slice/concat;
- `Path` как path-oriented text representation;
- `args`, `input`, `output`, `read`, `write`;
- `fs.list`, `fs.join`, `fs.is_dir`;
- deterministic cleanup для generated list/text owners.

Текущий index contract остаётся fail-soft и описан в language reference.

## Math runtime

Math core понижается через `math.h` и generated helper expressions:

- `PI`, `TAU`, `E`, `EPSILON`;
- `abs`, `min`, `max`, `clamp`;
- `floor`, `ceil`, `round`;
- `sin`, `cos`, `atan2`, `sqrt`, `root`;
- `deg_to_rad`, `rad_to_deg`;
- `dot`, `length`, `length_sq`, `normalize`, `distance`, `distance_sq`, `cross`;
- оператор степени `^` через `pow`.

## Memory runtime (`v1.2`, experimental)

- `Memory name = memory(size)` создаёт fixed-capacity region по умолчанию;
- `allow grow` добавляет новые chunks без `realloc` существующих buffers;
- `allow drop` сохраняется как явный policy bit, но не запускает скрытую
  reclamation живых значений;
- `memory.child(size)` получает fixed storage из активного parent-region;
- `memory.static(<literal>)` использует root-only static buffer;
- `place in` переключает thread-local active region;
- trailing `on error` обрабатывает overflow;
- `clear` сбрасывает region;
- semantic pass проверяет capability, escape и use-after-clear rules;
- runtime одинаково используется обычным кодом и native tasks без global race;
- dynamic/growth storage освобождается автоматически текущим scope owner.

## Ownership transfer (`v1.2`, experimental)

- `move` lower'ится для `Canvas`, `Window`, `Interrupt` и owning `Channel`;
- move-helper извлекает handle и обнуляет moved-from binding;
- owning параметры освобождают ресурс на normal fallthrough;
- early return освобождает только оставшихся текущих owners;
- `return move resource` переносит handle вызывающему коду без double cleanup.

## Task/Channel runtime (`v1.2`, experimental)

- `Task`, `Task(T)`, `run`, `wait`, `stop`, `stopping`;
- Win32 threads и pthread backend;
- typed argument/result contexts и generated trampolines;
- cooperative stop и обязательный join;
- bounded blocking `Channel(T)`;
- typed value-safe `send/receive` wrappers;
- mutex/condition-variable backpressure runtime;
- deterministic channel cleanup после task lifecycle.

CLI добавляет platform link flags, включая `-pthread` на POSIX.

## Time runtime (`v1.2`, experimental)

- `Time` и `Duration` lower'ятся в signed `i64` nanoseconds;
- literals `ms`, `s`, `min` вычисляются и overflow-check'ятся до C codegen;
- `now` использует `QueryPerformanceCounter` или `clock_gettime(CLOCK_MONOTONIC)`;
- `elapsed` возвращает monotonic duration;
- `sleep`/`delay` используют `Sleep` или retry вокруг `nanosleep`;
- runtime failure имеет код `SC-RT-320`.

## ByteSize runtime (`v1.2`, experimental)

- literals `b`, `kb`, `mb`, `gb` вычисляются с бинарными множителями и
  overflow-check'ятся до C codegen;
- `ByteSize` lower'ится в signed `int64_t` bytes;
- nominal arithmetic и comparisons проверяются semantic pass;
- `memory(ByteSize)` вычисляет capacity один раз;
- non-positive capacity отклоняется до преобразования signed значения к `size_t`;
- structs, Lists, Task и Channel используют value-safe `int64_t` representation.

## Angle runtime (`v1.2`, experimental)

- `deg/rad` literals вычисляются и finite-check'ятся до C codegen;
- `Angle` lower'ится в `double` radians;
- angle arithmetic lower'ится в обычные C double expressions после semantic checks;
- `sin/cos/atan2` используют `math.h` напрямую;
- `deg_to_rad` и `rad_to_deg` lower'ятся в явные expressions с `M_PI`;
- Lists, Task и Channel используют value-safe `double` representation.

## Vector runtime (`v1.2`, experimental)

- `Vec2`, `Vec3`, `Vec4` lower'ятся в C structs из `double` components;
- typed structural literals становятся designated initializers;
- arithmetic, dot products, lengths, normalization and distance используют
  небольшие статические helpers без hidden allocation;
- zero normalization возвращает zero-initialized struct;
- `cross` генерируется только для `Vec3`;
- Lists, Task и Channel используют value-safe struct representation.

## Canvas runtime (`v1.2`, experimental)

- `Color` lower'ится в RGBA8 struct, `Rect` — в четыре `double`;
- `Canvas` является owning software framebuffer и освобождается
  детерминированно при выходе из scope;
- `clear/pixel/line/rect/fill_rect/circle/fill_circle` lower'ятся в небольшой
  allocation-free rasterizer с source-over alpha blending и clipping;
- `checksum` даёт deterministic headless regression seam;
- `Window` является отдельным linear resource;
- `windows.open` и `present(direct canvas)` используют Win32/GDI backend;
- headless Canvas не включает Window runtime, а Windows toolchain добавляет
  `gdi32` только для соответствующего target path;
- `Color`/`Rect` value-safe, `Canvas`/`Window` запрещены в Task/Channel и
  передаются функциям только через explicit borrow.

## Platform scope

Release matrix проверяет generated C на:

- Windows MinGW и MSVC;
- Linux GCC и Clang;
- macOS host compiler;
- GCC ThreadSanitizer для concurrency runtime.

ESP32/FreeRTOS, AVR и другие embedded runtimes пока являются отдельным target
roadmap, а не скрытым обещанием desktop C backend.

## Не реализовано в backend

- hardware interrupt backends поверх periodic host MVP;
- wall-clock/calendar/timezone API;
- task groups, `select`, channel timeout/cancellation;
- shared mutable state primitives;
- Canvas events, text/images, transforms, Matrix2D и non-Windows presenters;
- automatic `allow drop` reclamation и user drop hooks;
- перенос ownership для самого `Memory` и `Task`;
- generic units algebra, `Timer`, matrices, SIMD lowering, swizzles и расширенная vector algebra;
- re-exports и module-name imports.

## Инварианты generated C

- одинаковый AST должен давать детерминированный C output;
- unsupported surface отклоняется semantic/codegen diagnostic, а не молча
  превращается в другое поведение;
- generated C должен оставаться пригодным для диагностики и sanitizer runs;
- shape tests закрепляют важные runtime hooks, native e2e собирает и запускает
  representative programs.
