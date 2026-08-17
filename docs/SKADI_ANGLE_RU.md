# Углы в Skadi

Статус: experimental `Angle` MVP текущей линии `v1.2`.

`Angle` - nominal-тип угла. Он хранится как `f32` radians, но не является alias
для `Float`: единица и назначение значения остаются видимыми в исходном коде и
проверяются semantic pass.

## Быстрый пример

```skadi
new Angle heading = 45deg
new Float x = cos(heading)
new Float y = sin(heading)
new Angle measured = atan2(y, x)
new Float measured_degrees = rad_to_deg(measured)
```

## Литералы

Поддерживаются целые и дробные joined literals:

```skadi
new Angle quarter_turn = 90deg
new Angle phase = 0.25rad
```

Литерал пишется слитно. `90 deg` и `0.25 rad` не входят в контракт. Значение
должно помещаться в finite `f32`; `NaN` и infinity не создаются literal parser.
Отрицательный угол записывается обычным unary minus: `-45deg`.

## Арифметика

| Выражение | Результат |
|---|---|
| `Angle + Angle` | `Angle` |
| `Angle - Angle` | `Angle` |
| `Angle * Int/Float` | `Angle` |
| `Int/Float * Angle` | `Angle` |
| `Angle / Int/Float` | `Angle` |
| `Angle / Angle` | `Float` |
| сравнения двух `Angle` | `Bool` |

Сложение угла с `Int/Float`, scalar делённый на угол, `%`, `div`, `mod` и
неявные `Float <-> Angle` conversions запрещены. Деление на нулевой scalar не
получает отдельной runtime-политики в этом MVP и следует поведению C `float`.

## Создание и преобразование

```skadi
new Angle literal = 30deg
new Angle converted = deg_to_rad(30.0)
new Angle measured = atan2(1.0, 1.0)
new Float degrees = rad_to_deg(measured)
```

- `deg_to_rad(Int/Float)` возвращает `Angle`;
- `atan2(Float, Float)` возвращает `Angle`;
- `rad_to_deg(Angle)` возвращает `Float` в градусах;
- `as_radians(Angle)` возвращает `Float` в радианах;
- literal `...rad` создаёт `Angle` непосредственно из radians.

`sin(Angle)`, `cos(Angle)` и `tan(Angle)` возвращают `Float`. Обычный numeric
аргумент не считается неявными радианами. `asin`, `acos` и `atan` принимают
numeric и возвращают `Angle`; `normalize_angle(Angle)` приводит угол к диапазону
`[-PI, PI)`.

Обычные math domain failures следуют IEEE 754 / C `math.h`: например,
`asin(2.0)` возвращает `NaN`. Результат проверяется через `is_nan`, `is_finite`
или `is_infinite`, без обязательного `on error`.

## Контейнеры и concurrency

`Angle` является value-safe типом. Его можно использовать в structs, Lists,
function arguments/results, `Task(Angle)` и `Channel(Angle)`.

```skadi
fn calculate() returns Angle {
    return 90deg
}

Task(Angle) calculation_task = run calculate()
new Angle result = wait calculation_task
new Angle List headings = [45deg, result]
```

## Ограничения MVP

- нет общей dimensional algebra и angular velocity;
- нет implicit `Float <-> Angle` conversions;
- нет автоматического `output(Angle)`: единицу нужно выбрать явно;
- нет special fixed-point/embedded representation;
- runtime-результаты scalar arithmetic не получают отдельной finite-проверки.

Проверяемый showcase: `benchmarks/bench_15_angle_navigation.skd`.
