# Короткие примеры Skadi

Эта страница дополняет [руководство для новичка](getting-started.md) небольшими
рецептами. Для точных ограничений каждой конструкции используйте
[справочник языка](language-reference.md) и [статус синтаксиса](syntax-status.md).

## Числа и операторы

```skadi
new Int quotient = 17 div 2
new Int remainder = 17 mod 2
new Bool odd = not (remainder == 0)
new Bool exactly_one = odd xor false
new Float squared = 3.0 ^ 2.0
```

`div` и `mod` работают с целыми числами, `not`, `and`, `or`, `xor` - с
логическими значениями. Оператор `^` используется для возведения в степень.

## Fixed-width числа

```skadi
new i16 temperature_raw = -125
new u32 packet_count = 1000
new f32 sensor_ratio = 0.25
```

Используйте `Int` и `Float` по умолчанию, а fixed-width типы - когда разрядность
является частью контракта данных или target-платформы.

## Список как изменяемая коллекция

```skadi
new i32 List values = [2, 4, 6]
values.push(8)

new Int total = 0
iterate values as value {
    total = total + value
}
```

Извлечение последнего элемента явно обрабатывает пустой список:

```skadi
new i32 last = 0
last = values.pop() on error {
    last = -1
}
```

## Функция с явной ошибкой

```skadi
label ErrorCode {
    Ok
    InvalidValue
}

danger fn positive_half(Int value) returns Int {
    if value < 0 {
        return error InvalidValue
    }

    return value div 2
}

new Int result = 0
result = positive_half(10) on error {
    result = 0
}
```

Первый вариант `ErrorCode` всегда должен быть `Ok`. `on error` применяется
только к `danger fn` и операциям с собственным error-контрактом, например
`List.pop()`.

## Структура, метод и скрытое поле

```skadi
struct Counter {
    hide Int changes
    Int value

    fn add(Int delta) returns Int {
        my.value = my.value + delta
        my.changes = my.changes + 1
        return my.value
    }
}

new Counter counter = {changes = 0, value = 5}
new Int next = counter.add(2)
```

`hide` запрещает внешний доступ к полю, но методы той же структуры продолжают
работать с ним через `my.field`.

## Два файла и квалифицированное имя

`math_utils.skd`:

```skadi
fn add(Int a, Int b) returns Int {
    return a + b
}

local fn implementation_detail() returns Int {
    return 0
}
```

`main.skd`:

```skadi
import "./math_utils.skd"

new Int direct = add(2, 3)
new Int explicit = math_utils.add(4, 5)
```

Публичные символы видны только через прямой import. `local`-объявления за
пределы файла не экспортируются.

## Файлы и пути

```skadi
new Path root = "."
new Path List entries = fs.list(root)

iterate entries as entry {
    new Path full = fs.join(root, entry)
    if fs.is_dir(full) {
        output(full)
    }
}

new Text body = read("input.txt")
new Int written = write("output.txt", body)
```

Текущие `read`, `write` и `fs.*` не являются `danger` builtins, поэтому trailing
`on error` к ним не добавляется.

## Математика и углы

```skadi
new Angle angle = 45deg
new Float x = cos(angle)
new Float y = sin(angle)
new Float length = sqrt((x * x) + (y * y))
new Angle measured = atan2(y, x)
new Float heading_degrees = rad_to_deg(measured)
new Float safe_heading = clamp(heading_degrees, 0.0, 360.0)
```

Math core и константы `PI`, `TAU`, `E`, `EPSILON` входят в stable base `v1.1`.
Nominal `Angle` и literals `deg/rad` относятся к experimental `v1.2`; полный
контракт описан на странице [Углы](angle.md).

## Canvas v0 (`v1.2`, experimental)

```skadi
Canvas frame = canvas(64, 48)
new Color background = color_hex("#1d1f21", 255)
new Rect panel = rect(4.0, 4.0, 56.0, 40.0)
new Vec2 center = {x = 32.0, y = 24.0}

frame.clear(background)
frame.fill_rect(panel, Color.terminal_blue)
frame.circle(center, 12.0, Color.terminal_bright_yellow)
output(frame.checksum())
```

Этот пример работает без окна и подходит для тестов. На Windows кадр можно
показать через `Window window = windows.open(...)` и
`window.present(direct frame)`. API, палитра и resource rules описаны на
странице [Canvas и Visual Core](canvas.md).

## Векторы (`v1.2`, experimental)

```skadi
new Vec2 position = {x = 2.0, y = 1.0}
new Vec2 target = {x = 8.0, y = 9.0}
new Vec2 heading = normalize(target - position)
new Float route_length = distance(position, target)
output(heading.x)
output(route_length)
```

Размерность входит в тип, components имеют `f64` representation. Полный
контракт и список builtins описаны на странице [Векторы](vectors.md).

## Время и длительности (`v1.2`, experimental)

```skadi
new Duration budget = 5ms
new Time started_at = now()
sleep(budget)
new Duration measured = elapsed(started_at)
new Bool completed = measured >= budget
```

`Time` использует monotonic clock. `Duration` не смешивается с `Int` неявно;
поддерживаются целые literals `ms`, `s`, `min`. Полный контракт описан на
странице [Время и длительности](time-duration.md).

## Размеры памяти (`v1.2`, experimental)

```skadi
new ByteSize payload = 2kb
new ByteSize overhead = 512b
new ByteSize capacity = payload + overhead
new Bool enough = capacity >= 2kb

Memory scratch_memory = memory(capacity)
```

`ByteSize` не смешивается с `Int/Float` неявно. Поддерживаются целые joined
literals `b`, `kb`, `mb`, `gb`; `memory(...)` принимает полноценное выражение
`ByteSize`. Полный контракт описан на странице [Размеры памяти](byte-size.md).

## Memory (`v1.2`, experimental)

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

Region имеет фиксированную ёмкость. Dynamic payload из локальной `Memory` нельзя
возвращать или переносить в более долгоживущий owner.

## Task с результатом (`v1.2`, experimental)

```skadi
fn load_status() returns Text {
    return "ready"
}

Task(Text) status_task = run load_status()
new Text status = wait status_task
output(status)
```

Handle нельзя игнорировать: каждый owning `Task` должен ровно один раз дойти до
`wait` на всех путях выполнения.

## Channel между задачами (`v1.2`, experimental)

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

`Channel(T)` является bounded blocking FIFO. Практические схемы с несколькими
workers, остановкой и повторным запуском собраны в
[руководстве по многопоточности](concurrency.md).

## Проверяемые исходники

Небольшой объединённый пример находится в
`examples/language/01_small_features.skd` и проходит compiler pipeline в
регрессионном тесте. Более крупные сценарии находятся в `benchmarks/` и описаны
на странице [Showcase-программы](showcases.md).
