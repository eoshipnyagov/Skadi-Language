# Vectors in Skadi

Status: experimental `Vec2/Vec3/Vec4` MVP in the current `v1.2` line.

`Vec2`, `Vec3`, and `Vec4` are built-in value types with `f32` components. The
dimension is part of the type, so vectors of different dimensions never mix
implicitly.

## Construction and components

Vectors use typed structural literals:

```skadi
new Vec2 position = {x = 10.0, y = 20.0}
new Vec3 direction = {x = 1.0, y = 0.0, z = 0.0}
new Vec4 weights = {x = 1.0, y = 2.0, z = 3.0, w = 4.0}
```

The field set must be exact: `x/y`, `x/y/z`, or `x/y/z/w`. Components accept
`Int` or `Float`, are read as `Float`, and can be assigned directly.

## Operations

The MVP supports same-dimension `+` and `-`, unary `-`, multiplication by an
`Int/Float` scalar in either order, and division by an `Int/Float` scalar.
Component-wise vector multiplication/division and vector comparisons are not
supported.

## Built-ins

| Function | Arguments | Result |
|---|---|---|
| `dot` | two same-dimension vectors | `Float` |
| `length`, `length_sq` | `VecN` | `Float` |
| `normalize` | `VecN` | the same `VecN` |
| `distance`, `distance_sq` | two same-dimension vectors | `Float` |
| `cross` | two `Vec3` values | `Vec3` |

`normalize` returns a zero vector for zero input.

## Value boundaries and MVP limits

Vectors are value-safe in structs, Lists, function arguments/results,
`Task(VecN)`, and `Channel(VecN)`. They lower to small C structs containing
`float` components.

Matrices, generic vectors, SIMD-specific lowering, swizzling, user-defined
operator overloading, and separate `f32` vectors are outside this MVP.

Verified showcase: `benchmarks/bench_16_vector_navigation.skd`.
