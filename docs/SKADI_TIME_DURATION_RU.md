# Время и длительности в Skadi

Статус: experimental runtime MVP текущей линии `v1.2`.

Этот слой добавляет два nominal-типа:

- `Time` - точка на monotonic clock;
- `Duration` - длительность или разница между двумя `Time`.

Они не являются aliases для `Int` и не смешиваются с обычными числами неявно.

## Быстрый пример

```skadi
new Time started_at = now()
sleep(5ms)
new Duration measured = elapsed(started_at)
new Bool completed = measured >= 5ms
output(completed)
```

## Литералы `Duration`

Первый MVP поддерживает целочисленные literals:

```skadi
new Duration debounce = 25ms
new Duration timeout = 2s
new Duration maintenance = 3min
```

Число и единица пишутся слитно. `1.5s` и `1 s` не входят в текущий контракт.

Поддерживаемые единицы:

- `ms` - миллисекунды;
- `s` - секунды;
- `min` - минуты.

Compiler проверяет переполнение при переводе literal во внутреннее представление.

## Внутреннее представление

`Time` и `Duration` понижаются в signed `i64` nanoseconds. Это implementation
contract текущего runtime, но не разрешение использовать обычный `Int` вместо
этих типов.

`Time` использует monotonic clock:

- значение подходит для интервалов, deadline и измерений;
- значение не является Unix timestamp;
- из него нельзя получить календарную дату;
- origin clock намеренно не определён публичным API.

## Builtins

### `now() returns Time`

```skadi
new Time checkpoint = now()
```

Возвращает текущую точку monotonic clock.

### `elapsed(Time) returns Duration`

```skadi
new Time started_at = now()
new Duration spent = elapsed(started_at)
```

Эквивалентен безопасному измерению `now() - started_at`. Если platform clock
нарушит monotonic contract, runtime возвращает нулевую длительность, а не
отрицательное значение.

### `sleep(Duration) returns Int` и `delay(Duration) returns Int`

```skadi
sleep(10ms)
delay(1s)
```

На desktop host обе функции блокируют текущий OS thread, сейчас имеют одинаковую
семантику и при успешном завершении возвращают `0`. Разные embedded-политики для
cooperative `delay` могут появиться только после отдельного target contract.

Нулевая или отрицательная вычисленная длительность завершается сразу.

## Допустимая арифметика

```skadi
new Duration frame = 16ms + 500ms
new Duration remaining = frame - 5ms

new Time started_at = now()
new Time deadline = started_at + remaining
new Duration window = deadline - started_at
new Time earlier = deadline - 1ms
```

Текущий контракт:

| Выражение | Результат |
|---|---|
| `Duration + Duration` | `Duration` |
| `Duration - Duration` | `Duration` |
| `Time + Duration` | `Time` |
| `Duration + Time` | `Time` |
| `Time - Duration` | `Time` |
| `Time - Time` | `Duration` |

Сравнения `==`, `!=`, `<`, `<=`, `>`, `>=` разрешены между двумя `Time` или
между двумя `Duration`.

Не поддерживаются:

- `Time + Time`;
- арифметика между `Duration` и `Int/Float`;
- умножение, деление и возведение длительности в степень;
- неявное присваивание `Int -> Duration` или `Duration -> Time`.

## Функции, списки и concurrency

`Time` и `Duration` являются value-safe types. Их можно использовать в полях
структур, списках, аргументах, return values, `Task(T)` и `Channel(T)`:

```skadi
fn measure(Duration budget) returns Duration {
    new Time started_at = now()
    sleep(budget)
    return elapsed(started_at)
}

Task(Duration) measurement_task = run measure(5ms)
new Duration measured = wait measurement_task

Channel(Duration) samples = channel(1)
samples.send(measured)
new Duration received = samples.receive()
```

## Platform runtime

Текущий C backend использует:

- Windows: `QueryPerformanceCounter` и `Sleep`;
- POSIX: `clock_gettime(CLOCK_MONOTONIC)` и interrupt-safe retry вокруг `nanosleep`.

Runtime failure monotonic clock или sleep завершается диагностикой `SC-RT-320`.
ESP32/FreeRTOS backend пока не реализован; desktop API не следует принимать за
готовый embedded timing contract.

## Текущие ограничения

- нет wall-clock, calendar/date/timezone API;
- нет `Timer`;
- нет literals `ns`, `us`, `h`;
- нет fractional duration literals;
- нет general units algebra;
- `output(Time/Duration)` не добавляет скрытое форматирование;
- timeout-варианты `wait`, `send`, `receive` ещё не связаны с `Duration`.

Проверяемый пример: `benchmarks/bench_13_time_budget.skd`.
