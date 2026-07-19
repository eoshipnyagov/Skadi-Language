# Angles in Skadi

Status: experimental `Angle` MVP in the current `v1.2` line.

`Angle` is a nominal angle type stored as `f64` radians. It is not an alias for
`Float`; source units and semantic intent remain explicit.

## Quick example

```skadi
new Angle heading = 45deg
new Float x = cos(heading)
new Float y = sin(heading)
new Angle measured = atan2(y, x)
new Float measured_degrees = rad_to_deg(measured)
```

## Literals and representation

Integer and fractional joined literals are supported:

```skadi
new Angle quarter_turn = 90deg
new Angle phase = 0.25rad
```

Spaced forms are rejected. Literal conversion must produce a finite `f64`.
Negative angles use unary minus, for example `-45deg`.

## Operations

- `Angle +/- Angle -> Angle`;
- `Angle * scalar`, `scalar * Angle`, and `Angle / scalar -> Angle`;
- `Angle / Angle -> Float`;
- same-type comparisons return `Bool`;
- implicit numeric mixing and `Float <-> Angle` conversions are rejected.

## Math integration

- `deg_to_rad(Int/Float) -> Angle`;
- `atan2(numeric, numeric) -> Angle`;
- `rad_to_deg(Angle) -> Float`;
- `sin(Angle)` and `cos(Angle)` return `Float`.

For `v1.1` compatibility, `sin/cos` temporarily still accept numeric raw
radians. Canonical `v1.2` code uses `Angle`.

`Angle` is value-safe in structs, Lists, `Task(Angle)`, and `Channel(Angle)`.
The MVP does not include dimensional algebra, angle normalization, angular
velocity, automatic `output(Angle)`, or a fixed-point embedded representation.

Compile-checked showcase: `benchmarks/bench_15_angle_navigation.skd`.
