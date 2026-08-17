# Skadi Syntax Status

Date: 2026-08-17

## Stable `v1.1`

- declarations, assignments, functions, `danger fn`, `ErrorCode`, and `on error`;
- `if`, `when`, `while`, `loop`, `iterate`, `for ... in`;
- structs, methods, hidden fields, Text, Path, List;
- numeric `label`, symbolic `tag`, nominal values, and the special `ErrorCode` contract;
- relative path imports, direct-import visibility, qualified symbols;
- I/O, filesystem, scalar math, constants, formatter, CLI, and TUI.

## Experimental executable `v1.2`

- fixed, segmented-growing, child, and root-static Memory regions;
- native Task/Channel on Win32/pthread;
- Time/Duration and monotonic runtime;
- ByteSize, Angle, Vec2/Vec3/Vec4 and vector math.
- `direct` mutable and `view` read-only call-scoped borrows;
- explicit `move` ownership transfer and resource-returning factories;
- periodic `Interrupt` sources and `on interrupt`;
- Channel `try_send`, `close`/drain, cancellation-aware blocking operations,
  `send_for`/`receive_for`, and contextual `timed_out`;
- path-sensitive `wait task for Duration on error { ... }`;
- Canvas v0 with a Win32 presenter and portable headless rasterizer.

## Compatibility

- `bool`/`char` aliases;
- `for ... in` works, while `iterate ... as ...` is showcase-canonical;
- legacy typed returns are reformatted to `returns`;
- compound assignments are expanded;
- C-style `for` is parsed/formatted but rejected semantically.

General `label Name` and `tag Name` declarations are complete nominal types.
They work in variables, function parameters/results, comparisons, and `when`;
inside a nominal `when`, `is Ready` is the short form of `Type.Ready`.

`output` accepts one or more printable scalar/Text values, writes them without
implicit separators, and appends one newline.

Math includes scalar selection/rounding, `sign`, `trunc`, `fract`, interpolation
and range mapping, complete basic trigonometry with nominal `Angle`, IEEE result
checks, and explicit angle/unit representation access. Duration literals cover
`ns/us/ms/s/min/h`; ByteSize literals cover binary `b/kb/mb/gb/tb`.

## Reserved or future

- automatic `allow drop` reclamation;
- package-name imports and re-exports;
- Channel `select`/`try_receive`, and async/task groups;
- ESP32/RTOS backend;
- Matrix2D, Canvas events/text/images, non-Windows presenters, and generic units/vectors.

The [quick reference](language-quick-reference.en.md) lists every public form and
builtin.

`fixed` and `const` are ordinary identifiers. `constant` is the only immutable
binding keyword.
