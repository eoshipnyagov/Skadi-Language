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
```

`abs`, `min`, `max`, `clamp` сохраняют целый результат, если все аргументы
целые. `floor`, `ceil`, `round`, `sqrt` и `root` возвращают `Float`.

## Углы

```skadi
new Angle heading = 90deg
new Float vertical = sin(heading)
new Angle direction = atan2(y, x)
new Float degrees = rad_to_deg(direction)
```

`deg_to_rad(number)` и `atan2(y, x)` возвращают `Angle`. `rad_to_deg` принимает
`Angle`. Numeric radians в `sin/cos` временно поддерживаются для совместимости
с `v1.1`, но новый код должен использовать `Angle`.

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
