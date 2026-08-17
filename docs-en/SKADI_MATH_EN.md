# Math

`PI`, `TAU`, `E`, and `EPSILON` are import-free `Float` constants.

Scalar functions: `abs`, `min`, `max`, `clamp`, `floor`, `ceil`, `round`,
`sign`, `trunc`, `fract`, `sqrt`, and `root`. Range helpers are `lerp`,
`inverse_lerp`, `remap`, and `smoothstep`. `fract(x)` is `x - floor(x)`, so
`fract(-1.25) == 0.75`; `lerp` deliberately does not clamp its interpolation
factor.

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

`sin`, `cos`, and `tan` require `Angle`; `asin`, `acos`, `atan`, and `atan2`
return `Angle`. Floating-point domain results follow IEEE 754 / `math.h`. Use
`is_nan`, `is_finite`, and `is_infinite` for explicit validation.

Vector functions: `dot`, `length`, `length_sq`, `normalize`, `distance`,
`distance_sq`, and Vec3-only `cross`.

```skadi
new Vec2 velocity = {x = 3.0, y = 4.0}
new Float speed = length(velocity)
new Vec2 direction = normalize(velocity)
```

Matrix2D, generic vectors, and a SIMD contract are not implemented.
