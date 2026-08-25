# C ABI и native C

Skadi может вызывать небольшие C API через явно объявленные функции. Текущий
MVP намеренно ограничен скалярами, by-value структурами и типизированными буферами: он подходит для
подключения проверенных C-функций, драйверных обёрток и постепенной проверки
FFI-дизайна, но не открывает raw pointers и не скрывает владение ресурсами.

## Минимальный пример

Готовый запускаемый проект находится в `examples/c-abi`; его объявления
вынесены в отдельный binding-модуль `src/sensor.skd`. Ниже показан сокращённый
однофайловый вариант.

`src/main.skd`:

```skadi
external fn c_add(i32 left, i32 right) returns i32

new i32 answer = c_add(20, 22)
output(answer)
```

`native/helper.c`:

```c
#include <stdint.h>

int32_t c_add(int32_t left, int32_t right) {
    return left + right;
}
```

`Skadi.toml`:

```toml
[native]
sources = ["native/helper.c"]
libraries = []
library_paths = []
```

Запуск выполняется обычным проектным потоком:

```powershell
skadi-cli check
skadi-cli run
```

`check` проверяет Skadi-сигнатуру и manifest. `build`/`run` дополнительно
проверяют native paths, компилируют C-файлы и выполняют линковку.

## Синтаксис объявления

```skadi
external fn name(Type argument, Type other) returns ReturnType
external fn notify()
```

У `external fn` нет тела. Отсутствие `returns` означает C `void`. Внешняя функция
вызывается как обычная типизированная функция Skadi, но не может быть напрямую
запущена через `run`: для задачи нужна Skadi-обёртка.

```skadi
external fn c_poll() returns i32

fn poll() returns i32 {
    return c_poll()
}

Task(i32) polling = run poll()
new i32 result = wait polling
```

## Ошибки через `external danger fn`

Внешний C-адаптер может использовать обычную модель ошибок Skadi:

```skadi
external danger fn sensor_read(i32 channel) returns i32

new i32 reading = 0
reading = sensor_read(2) on error {
    output("sensor read failed")
}
```

Соответствующий C ABI:

```c
int sensor_read(int32_t channel, int32_t *out) {
    if (out == 0) {
        return 1;
    }
    *out = 400;
    return 0;
}
```

`0` означает успех, ненулевой status запускает `on error`. При `returns T`
результат записывается через последний параметр `T *out`; без `returns` функция
имеет только status-result. Обычный вызов danger-функции без `on error`
отклоняется semantic analysis.

## Разрешённые типы

| Skadi | C |
|---|---|
| `i8`, `i16`, `i32`, `i64` | `int8_t`, `int16_t`, `int32_t`, `int64_t` |
| `u8`, `u16`, `u32`, `u64` | `uint8_t`, `uint16_t`, `uint32_t`, `uint64_t` |
| `f32` | `float` |
| `f64` | `double` |
| `Bool` | `bool` |
| `Char` | `char` |
| нет `returns` | `void` |

Для `external danger fn` C return type всегда `int`, а заявленный Skadi
return type становится последним `out`-параметром.

`Int`, `Float`, `Text`, `Path`, списки, обычные struct и specialized types не
входят в текущий ABI. Для границы C используйте fixed-width типы, явно
объявленный `external struct`, call-scoped `Buffer(T)` или opaque
`external resource`. ABI values передаются по значению; owning handle всегда
требует видимый `view`, `edit` или `move`.

## C-compatible структуры

`external struct` описывает value-структуру с обычным layout выбранного C
compiler:

```skadi
external struct SensorReading {
    i32 value
    f32 confidence
    Bool valid
}

external fn sensor_describe(i32 value) returns SensorReading
```

Соответствующая сторона C должна объявить поля в том же порядке и с теми же
типами:

```c
typedef struct {
    int32_t value;
    float confidence;
    bool valid;
} SensorReading;
```

Контракт первого среза:

- минимум одно поле;
- только fixed scalar ABI fields из таблицы выше;
- порядок полей сохраняется;
- используются обычные alignment/padding rules C compiler без `packed`;
- методы, `hide`, вложенные структуры, arrays, pointers и owning fields запрещены;
- аргументы и результаты `external fn` передаются по значению;
- `view`/`edit external struct`, `Buffer(Struct)` и layout attributes отложены.

## Opaque owning resources

`external resource` объявляет тип C handle, внутреннее устройство которого
Skadi не видит:

```skadi
external resource Sensor

external fn sensor_open(i32 channel) returns Sensor
external fn sensor_read(view Sensor sensor) returns i32
external danger fn sensor_adjust(edit Sensor sensor, i32 delta)
external danger fn sensor_close(move Sensor sensor)

new Sensor sensor = sensor_open(2)
new i32 value = sensor_read(view sensor)
sensor_adjust(edit sensor, 1) on error {
    output("adjust failed")
}
sensor_close(move sensor) on error {
    output("close failed")
}
```

На C-границе `Sensor` понижается в opaque `void *`. C adapter должен принимать
и возвращать совместимый pointer-sized handle; приводить его к внутреннему типу
можно только внутри native C source.

Контракт владения:

- factory, возвращающая `Sensor`, создаёт нового единственного owner;
- `view Sensor` разрешает синхронное чтение без передачи ownership;
- `edit Sensor` разрешает синхронное изменение без передачи ownership;
- `move Sensor` потребляет owner независимо от success/error status C-вызова;
- результат factory нельзя игнорировать или копировать;
- перед выходом из owning scope handle должен быть передан функции с
  `move Sensor` на каждом control-flow path;
- `external resource` нельзя хранить в `List`, struct, `Task` или `Channel` и
  нельзя преобразовывать в integer/raw pointer.
- в текущем срезе factory должна быть trusted non-danger `external fn`:
  `external danger fn ... returns Resource` отложена до typed declaration с
  `on error`, чтобы не вводить nullable/uninitialized handle.

Skadi намеренно не угадывает destructor по имени и не выполняет скрытый
`sensor_close`. Освобождающая функция является обычным явно объявленным
`external fn`; если она `danger`, call site обязан написать `on error`.

Skadi не читает C header и не может доказать, что независимое C-объявление
совпадает. Обе части должны собираться ABI-совместимыми toolchains и target
настройками; лучше держать binding и C adapter рядом и проверять native smoke.

## Типизированные буферы

`Buffer(T)` доступен только как параметр `external fn` и связывает типизированный
Skadi `T List` с обычной C-парой pointer + length:

```skadi
external fn checksum(view Buffer(u8) data) returns u32
external danger fn adjust(edit Buffer(u8) data, u8 delta)

new u8 List packet = [10, 20, 30]
new u32 sum = checksum(view packet)
adjust(edit packet, 1) on error {
    output("adjust failed")
}
```

```c
uint32_t checksum(const uint8_t *data, size_t data_length);
int adjust(uint8_t *data, size_t data_length, uint8_t delta);
```

- `view Buffer(T)` даёт C только чтение через `const T *`;
- `edit Buffer(T)` даёт исключительный mutable-доступ через `T *`;
- длина имеет C-тип `size_t` и измеряется в элементах, не в байтах;
- `T` должен быть разрешённым fixed scalar ABI type;
- `Buffer(T)` нельзя объявить как переменную, вернуть, сохранить в struct,
  вложить в список или передать через `run`;
- borrow заканчивается после синхронного C-вызова, поэтому C-код не должен
  сохранять указатель для последующего использования.

## Поля `[native]`

| Поле | Значение |
|---|---|
| `sources` | C-файлы проекта; только относительные пути к `.c` |
| `libraries` | Имена системных/предсобранных библиотек без shell flags |
| `library_paths` | Относительные каталоги поиска библиотек |

Пути считаются от корня проекта, не могут быть абсолютными и не могут содержать
`..`. Для GCC/Clang library path и имя понижаются в `-L...`/`-l...`; для MSVC —
в `/LIBPATH:...` и `.lib`. Произвольные compiler/linker flags через manifest не
поддерживаются.

```toml
[native]
sources = ["native/adapter.c", "native/checksum.c"]
libraries = ["sensor"]
library_paths = ["native/lib"]
```

Config editor TUI сохраняет эти поля, но в текущем MVP они редактируются
непосредственно в `Skadi.toml`.

## Практические правила

- Держите C-границу маленькой и оборачивайте доменное поведение в Skadi-функции.
- Используйте одинаковую C-сигнатуру и `external fn`; несовпадение ABI не может
  быть обнаружено Skadi-компилятором.
- Подключайте заголовки в C-файлах и проверяйте их обычным C compiler.
- Для cross-target сборки native sources и libraries тоже должны поддерживать
  выбранную платформу.
- Не передавайте C pointers как целые числа. Для owning handle используйте
  `external resource`; callbacks, borrowed external lifetimes и layout
  attributes остаются следующими FFI-этапами.

## Текущие границы

Не реализованы header import/generation, symbol aliases, calling-convention
attributes, packed/custom struct и enum layout, raw pointers, nullable values, callbacks, variadic
functions, C++ ABI и remote package/library resolver. Локальные Skadi package
dependencies уже разрешаются через `[dependencies]`, но не заменяют поиск и
версионирование native-библиотек. Это сознательная граница первого среза,
а не обещание автоматически поддерживать любую C-библиотеку.
