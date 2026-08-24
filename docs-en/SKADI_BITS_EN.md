# Integer Model and Bit Operations

## `Int` and fixed width

`Int` is intended for ordinary counters, indexes, and application arithmetic.
Its width is a target and project property:

```toml
[numeric]
int = "target"
```

`target` selects the target profile default. Current Windows and POSIX desktop
profiles use `i32`. A project may pin its ABI with `i8`, `i16`, `i32`, or `i64`.

The setting changes only `Int`. Types `i8/i16/i32/i64` and
`u8/u16/u32/u64` always retain their stated width. Use fixed-width types for
protocols, registers, file formats, FFI, and persistent data.
An `Int` literal that does not fit the selected width is rejected with
`SC-CG-302` instead of being silently narrowed by C.

`Float`, `Angle`, and `Vec2/Vec3/Vec4` components use `f32`; explicit `f64`
remains 64-bit and selects double-precision math without rounding a literal to
`f32` first. `Time`, `Duration`, `ByteSize`, `as_nanoseconds`, and `as_bytes`
use exact `i64` storage independently of `Int`.

## Ordinary integer arithmetic

The `+`, `-`, `*`, `/`, `div`, `mod`, `%`, `^`, unary `-`, and `++/--`
statements are checked. Overflow, division by zero, a negative integer exponent,
and negating the minimum signed value terminate with `SC-RT-350` instead of
reaching C undefined behavior. A literal zero divisor or negative integer
exponent is rejected during semantic analysis.

`Float/f32/f64` arithmetic retains IEEE behavior, including `NaN` and infinity.
Use `is_nan`, `is_finite`, and `is_infinite` when the result must be validated.

## Literals

Integer literals support decimal, binary, octal, and hexadecimal notation.
Underscores are visual separators:

```skadi
new u8 mask = 0b1111_0000
new u16 permissions = 0o0755
new u32 color = 0xFF_80_20_10
```

## Bit builtins

The import-free API is:

| Function | Result |
|---|---|
| `bit_and(a, b)` | Bitwise AND |
| `bit_or(a, b)` | Bitwise OR |
| `bit_xor(a, b)` | Bitwise XOR |
| `bit_not(value)` | Invert every bit |
| `bit_shift_left(value, count)` | Logical left shift |
| `bit_shift_right(value, count)` | Logical right shift |
| `bit_is_set(value, index)` | `Bool`: whether a bit is set |
| `bit_set(value, index)` | Copy with a bit set |
| `bit_clear(value, index)` | Copy with a bit cleared |
| `bit_toggle(value, index)` | Copy with a bit toggled |
| `bit_write(value, index, enabled)` | Set or clear from a `Bool` |

These functions accept only `i8/i16/i32/i64` and `u8/u16/u32/u64`.
Platform-sized `Int` is deliberately rejected so a register or mask cannot
change width with the target profile.

```skadi
new u8 flags = 0b0000_0001
flags = bit_set(flags, 3)
flags = bit_toggle(flags, 0)

if bit_is_set(flags, 3) {
    output("ready")
}
```

Every operation returns a value and never mutates an argument implicitly.
Binary operands must have the same fixed-width type. A literal may be combined
with a typed operand when it fits that type, for example
`bit_or(register_u32, 0xff)`.

Variables of different widths or signedness are not mixed implicitly. An integer
literal may initialize a fixed-width target only when it fits. Variables use an
explicit checked conversion with a mandatory handler:

```skadi
new u16 raw = 240
new u8 level = 0

level = as_u8(raw) on error {
    output("level is outside u8")
}
```

The complete family is `as_i8/as_i16/as_i32/as_i64` and
`as_u8/as_u16/as_u32/as_u64`. These functions accept any numeric value. A float
must be finite, integral, and in range. The argument is evaluated once, the
target is updated only on success, and no truncation, modulo conversion, or
silent overflow occurs. On failure the target retains its previous value.

`as_f32(value)` is a checked narrowing operation with mandatory `on error`; its
argument must be finite and fit `f32`. `as_f64(value)` is an ordinary safe
numeric widening operation and does not require a handler.

## Explicit wrapping arithmetic

Use `wrapping_add`, `wrapping_sub`, `wrapping_mul`, and `wrapping_neg` when
overflow is intentionally part of an algorithm. They accept only fixed-width
`i8..i64/u8..u64` values and preserve the low bits of the same width. `Int` is
deliberately rejected so modulo behavior cannot change with the target.

```skadi
new u8 sequence = 255
sequence = wrapping_add(sequence, 1) // 0

new i8 lowest = -128
lowest = wrapping_neg(lowest) // -128, the same bit pattern
```

Unary `-` is rejected for unsigned fixed-width values. Use `wrapping_neg` when
the intended result is specifically the two's-complement bit pattern.

## Shifts and bounds

Right shift is always logical, including for signed types: lowering first uses
the unsigned representation of the same width. Thus shifting an `i8` value of
`-1` right by one produces the bit pattern `0b0111_1111`, or `127`.

Indexes and shift counts must be in `0..width-1`. Constant mistakes are semantic
errors. Dynamic mistakes terminate with `SC-RT-340` instead of invoking C
undefined behavior.
