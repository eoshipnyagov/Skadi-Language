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
remains 64-bit. `Time`, `Duration`, `ByteSize`, `as_nanoseconds`, and `as_bytes`
use exact `i64` storage independently of `Int`.

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
`bit_or(word_u32, 0xff)`.

Variables of different widths or signedness are not mixed implicitly. A future
explicit numeric conversion contract must define widening, narrowing, and
overflow before such conversions become part of the language.

## Shifts and bounds

Right shift is always logical, including for signed types: lowering first uses
the unsigned representation of the same width. Thus shifting an `i8` value of
`-1` right by one produces the bit pattern `0b0111_1111`, or `127`.

Indexes and shift counts must be in `0..width-1`. Constant mistakes are semantic
errors. Dynamic mistakes terminate with `SC-RT-340` instead of invoking C
undefined behavior.
