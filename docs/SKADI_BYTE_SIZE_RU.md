# Размеры памяти в Skadi

Статус: experimental `ByteSize` MVP текущей линии `v1.2`.

`ByteSize` - nominal-тип количества байтов. Он не является alias для `Int` и не
смешивается с обычными числами неявно.

## Быстрый пример

```skadi
new ByteSize payload = 2kb
new ByteSize capacity = payload + 512b
new Bool enough = capacity >= 2kb

Memory scratch_memory = memory(capacity)
```

## Литералы

Поддерживаются целочисленные joined literals:

```skadi
new ByteSize header = 64b
new ByteSize page = 4kb
new ByteSize arena = 8mb
new ByteSize archive = 1gb
```

Множители бинарные:

| Единица | Байты |
|---|---:|
| `b` | 1 |
| `kb` | 1 024 |
| `mb` | 1 048 576 |
| `gb` | 1 073 741 824 |

Литерал пишется слитно. `1.5kb` и самостоятельное `1 kb` не входят в текущий
контракт. Старый `memory(8 mb)` принимается только как compatibility-вход Memory
MVP и formatter канонизирует его в `memory(8mb)`.

Compiler проверяет переполнение при переводе literal во внутреннее signed `i64`
представление.

## Арифметика

Разрешены:

| Выражение | Результат |
|---|---|
| `ByteSize + ByteSize` | `ByteSize` |
| `ByteSize - ByteSize` | `ByteSize` |
| сравнения двух `ByteSize` | `Bool` |

Смешивание с `Int/Float`, умножение, деление и неявные conversions запрещены.
Вычитание может дать отрицательный `ByteSize`; такое значение допустимо для
промежуточного расчёта, но не является валидной ёмкостью `Memory`.

## Связь с Memory

`memory(...)` теперь принимает выражение типа `ByteSize`:

```skadi
fn with_overhead(ByteSize payload) returns ByteSize {
    return payload + 1kb
}

new ByteSize capacity = with_overhead(4kb)
Memory assets_memory = memory(capacity) on error {
    output("memory capacity rejected")
}
```

Нулевая или отрицательная вычисленная ёмкость обрабатывается тем же runtime
failure path, что и ошибка выделения: выполняется trailing `on error`, а без него
runtime завершает программу диагностикой Memory.

Этот slice не добавляет `allow grow`, `allow drop`, `memory.child` или
`memory.static`.

## Контейнеры и concurrency

`ByteSize` является value-safe типом. Его можно использовать в structs, Lists,
function arguments/results, `Task(ByteSize)` и `Channel(ByteSize)`.

```skadi
fn calculate() returns ByteSize {
    return 4kb
}

Task(ByteSize) calculation_task = run calculate()
new ByteSize capacity = wait calculation_task
new ByteSize List options = [2kb, capacity]
```

## Ограничения MVP

- нет decimal `kB/MB` и отдельной IEC-нотации `KiB/MiB`;
- нет fractional literals;
- нет scalar multiplication/division;
- нет автоматического форматирования `ByteSize` для `output`;
- нет общей dimensional algebra;
- allocator policies остаются отдельным Memory roadmap.

Проверяемый showcase: `benchmarks/bench_14_byte_size_budget.skd`.
