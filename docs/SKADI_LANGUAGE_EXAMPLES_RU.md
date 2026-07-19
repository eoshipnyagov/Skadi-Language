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
new Float angle = deg_to_rad(45.0)
new Float x = cos(angle)
new Float y = sin(angle)
new Float length = sqrt((x * x) + (y * y))
new Float heading = rad_to_deg(atan2(y, x))
new Float safe_heading = clamp(heading, 0.0, 360.0)
```

Math core и константы `PI`, `TAU`, `E`, `EPSILON` входят в stable base `v1.1`.

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
