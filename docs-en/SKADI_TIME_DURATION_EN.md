# Time and Duration in Skadi

Status: experimental runtime MVP in the current `v1.2` line.

The time layer provides two nominal types:

- `Time` is a point on a monotonic clock;
- `Duration` is an interval or the difference between two `Time` values.

They are not aliases for `Int` and do not mix with ordinary numbers implicitly.

## Quick example

```skadi
new Time started_at = now()
sleep(5ms)
new Duration measured = elapsed(started_at)
new Bool completed = measured >= 5ms
output(completed)
```

## Duration literals

The first MVP accepts joined integer literals:

```skadi
new Duration debounce = 25ms
new Duration timeout = 2s
new Duration maintenance = 3min
new Duration hardware_tick = 1ns
new Duration sensor_pulse = 10us
new Duration session = 1h
```

Supported units are `ns`, `us`, `ms`, `s`, `min`, and `h`. `us` is the portable
ASCII spelling for microseconds. Fractional values such as `1.5s` and
spaced forms such as `1 s` are not part of the current contract. Literal
conversion to the internal representation is overflow-checked.

## Runtime model

Both values lower to signed `i64` nanoseconds. `Time` is monotonic and has no
public epoch: it is suitable for intervals and deadlines, not dates or Unix
timestamps.

Nanoseconds are representation precision, not a promise about physical wait
resolution. `sleep(1ns)` is rounded by the backend to the resolution of the
platform timer.

Builtins:

- `now() returns Time`;
- `elapsed(Time) returns Duration`;
- `sleep(Duration) returns Int`;
- `delay(Duration) returns Int`.

On desktop hosts, `sleep` and `delay` currently have the same blocking behavior
and return `0` after successful completion.

## Arithmetic

Supported combinations:

| Expression | Result |
|---|---|
| `Duration + Duration` | `Duration` |
| `Duration - Duration` | `Duration` |
| `Duration * Int`, `Int * Duration` | `Duration` |
| `Duration / Int` | `Duration`, using integer nanosecond division |
| `Duration / Duration` | `Float` |
| `Time + Duration` | `Time` |
| `Duration + Time` | `Time` |
| `Time - Duration` | `Time` |
| `Time - Time` | `Duration` |

Comparisons are supported between values of the same nominal type.
`as_nanoseconds(Duration) returns i64` exposes the exact representation.
Addition/subtraction with ordinary numbers, `Float` scaling, `Time + Time`, and
implicit numeric conversions are rejected.

## Containers and concurrency

`Time` and `Duration` are value-safe. They can be used in structs, lists,
function arguments/results, `Task(T)`, and `Channel(T)`.

```skadi
fn measure(Duration budget) returns Duration {
    new Time started_at = now()
    sleep(budget)
    return elapsed(started_at)
}

Task(Duration) measurement_task = run measure(5ms)
new Duration measured = wait measurement_task
```

## Platforms and limits

The C runtime uses `QueryPerformanceCounter`/`Sleep` on Windows and
`clock_gettime(CLOCK_MONOTONIC)`/`nanosleep` on POSIX. Runtime failures use
`SC-RT-320`.

Wall-clock/calendar APIs, `Timer`, fractional literals, and an
ESP32/FreeRTOS backend remain future work. Channel `send_for`/`receive_for` and
`wait task for Duration` use the nominal duration as their bounded-wait contract.

Compile-checked showcase: `benchmarks/bench_13_time_budget.skd`.
