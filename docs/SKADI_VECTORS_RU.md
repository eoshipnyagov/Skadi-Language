# Векторы в Skadi

Статус: experimental `Vec2/Vec3/Vec4` MVP текущей линии `v1.2`.

`Vec2`, `Vec3` и `Vec4` - встроенные value-типы с компонентами `f32`. Размерность
является частью типа: значения разных размерностей не смешиваются неявно.

## Создание и компоненты

Вектор создаётся типизированным структурным литералом:

```skadi
new Vec2 position = {x = 10.0, y = 20.0}
new Vec3 direction = {x = 1.0, y = 0.0, z = 0.0}
new Vec4 weights = {x = 1.0, y = 2.0, z = 3.0, w = 4.0}
```

Набор полей должен быть точным: `x/y` для `Vec2`, `x/y/z` для `Vec3` и
`x/y/z/w` для `Vec4`. Компоненты принимают `Int` или `Float` и читаются как
`Float`:

```skadi
new Float horizontal = position.x
position.y = 24.0
```

Конструкторная форма `Vec3(...)` и swizzling (`value.xy`) в MVP не входят.

## Операции

Поддерживаются:

- `VecN + VecN` и `VecN - VecN` для одинаковой размерности;
- unary `-VecN`;
- `VecN * Int/Float` и `Int/Float * VecN`;
- `VecN / Int/Float`.

```skadi
new Vec2 delta = target - position
new Vec2 halfway = position + (delta * 0.5)
```

Покомпонентные `VecN * VecN`, `VecN / VecN`, ordering и equality operators не
поддерживаются. Деление на нулевой scalar следует обычному поведению C `float`.

## Встроенные функции

| Функция | Аргументы | Результат |
|---|---|---|
| `dot` | два `VecN` одной размерности | `Float` |
| `length` | `VecN` | `Float` |
| `length_sq` | `VecN` | `Float` |
| `normalize` | `VecN` | тот же `VecN` |
| `distance` | два `VecN` одной размерности | `Float` |
| `distance_sq` | два `VecN` одной размерности | `Float` |
| `cross` | два `Vec3` | `Vec3` |

```skadi
new Vec3 east = {x = 1.0, y = 0.0, z = 0.0}
new Vec3 north = {x = 0.0, y = 1.0, z = 0.0}
new Vec3 up = cross(east, north)
new Vec3 unit_up = normalize(up)
new Float separation = distance(east, north)
```

`normalize` возвращает нулевой вектор для нулевого входа. Это исключает
непредсказуемый `NaN` в обычном безопасном сценарии.

## Value semantics и границы

Векторы копируются по значению и являются value-safe. Их можно использовать в
структурах, Lists, аргументах и результатах функций, `Task(VecN)` и
`Channel(VecN)`.

```skadi
fn axis() returns Vec3 {
    return {x = 1.0, y = 0.0, z = 0.0}
}

Task(Vec3) axis_task = run axis()
new Vec3 result = wait axis_task
new Vec3 List axes = [result]
```

`output(VecN)` намеренно не добавлен: выводите нужные компоненты явно.

## C lowering и границы MVP

Каждый тип lower'ится в небольшой C `struct` из `float`, а операции - в
статические helper-функции. `length`, `normalize` и `distance` используют
`math.h`.

В MVP не входят matrices, generic vectors, SIMD-specific lowering, swizzling,
пользовательский operator overloading и отдельные `f32`-векторы.

Проверяемый showcase: `benchmarks/bench_16_vector_navigation.skd`.
