# Математика

Math core доступен без imports.

## Константы

`PI`, `TAU`, `E`, `EPSILON` имеют тип `Float`.

```skadi
new Float circle = TAU
new Float tolerance = EPSILON
```

## Scalar math

```skadi
new Int magnitude = abs(-4)
new Float bounded = clamp(sensor, 0.0, 1.0)
new Float diagonal = sqrt(2.0)
new Float cube = root(27.0, 3.0)
new Int direction = sign(-12)
new Float fraction = fract(-1.25)
new Float mixed = lerp(10.0, 20.0, 0.25)
new Float voltage = remap(sensor, 0.0, 4095.0, 0.0, 3.3)
new Float fade = smoothstep(0.2, 0.8, signal)
```

`abs`, `min`, `max`, `clamp` сохраняют целый результат, если все аргументы
целые. `floor`, `ceil`, `round`, `trunc`, `fract`, `sqrt` и `root` возвращают
`Float`. `fract(x)` определён как `x - floor(x)`, поэтому `fract(-1.25) == 0.75`.

`lerp(a, b, t)` не ограничивает `t`; `inverse_lerp(a, b, value)` возвращает
позицию значения внутри диапазона; `remap(value, in_a, in_b, out_a, out_b)`
переносит её в другой диапазон. `smoothstep(edge_a, edge_b, value)` ограничивает
нормализованное значение диапазоном `0..1` и применяет плавную cubic-кривую.

## Углы

```skadi
new Angle heading = 90deg
new Float vertical = sin(heading)
new Float slope = tan(heading)
new Angle direction = atan2(y, x)
new Angle measured = asin(0.5)
new Angle wrapped = normalize_angle(direction)
new Float degrees = rad_to_deg(direction)
new Float radians = as_radians(direction)
```

`deg_to_rad(number)` и `atan2(y, x)` возвращают `Angle`. `rad_to_deg` принимает
`Angle`. `sin`, `cos` и `tan` требуют `Angle`; `asin`, `acos`, `atan` и `atan2`
возвращают `Angle`.

Ошибочные вещественные области следуют IEEE 754 и `math.h`, а не `on error`.
Используйте `is_nan`, `is_finite` и `is_infinite`, когда результат необходимо
проверить явно.

## Векторы

```skadi
new Vec2 velocity = {x = 3.0, y = 4.0}
new Float speed = length(velocity)
new Vec2 direction = normalize(velocity)
```

Vector builtins: `dot`, `length`, `length_sq`, `normalize`, `distance`,
`distance_sq`; `cross` принимает только два `Vec3`.

Полные bounded-контракты: [углы](angle.md) и [векторы](vectors.md).
`Matrix2D`, SIMD contract и generic vectors пока не реализованы.
