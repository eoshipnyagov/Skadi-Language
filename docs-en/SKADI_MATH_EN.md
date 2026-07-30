# Math

`PI`, `TAU`, `E`, and `EPSILON` are import-free `Float` constants.

Scalar functions: `abs`, `min`, `max`, `clamp`, `floor`, `ceil`, `round`,
`sqrt`, and `root`.

```skadi
new Angle heading = 90deg
new Float vertical = sin(heading)
new Angle direction = atan2(y, x)
new Float degrees = rad_to_deg(direction)
```

Vector functions: `dot`, `length`, `length_sq`, `normalize`, `distance`,
`distance_sq`, and Vec3-only `cross`.

```skadi
new Vec2 velocity = {x = 3.0, y = 4.0}
new Float speed = length(velocity)
new Vec2 direction = normalize(velocity)
```

Matrix2D, generic vectors, and a SIMD contract are not implemented.
