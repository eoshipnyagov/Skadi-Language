# Skadi Syntax Status

Date: 2026-07-30

## Stable `v1.1`

- declarations, assignments, functions, `danger fn`, `ErrorCode`, and `on error`;
- `if`, `when`, `while`, `loop`, `iterate`, `for ... in`;
- structs, methods, hidden fields, Text, Path, List;
- the special `label ErrorCode` contract;
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
- Canvas v0 with a Win32 presenter and portable headless rasterizer.

## Compatibility

- `bool`/`char` aliases;
- `for ... in` works, while `iterate ... as ...` is showcase-canonical;
- legacy typed returns are reformatted to `returns`;
- compound assignments are expanded;
- C-style `for` is parsed/formatted but rejected semantically.

General `label Name` declarations are partial: parser/module visibility works,
but only `ErrorCode` has a complete value/type/C-lowering contract.

## Reserved or future

- `fixed`, `const`;
- automatic `allow drop` reclamation;
- named/aliased imports and packages;
- channel close/select/timeouts and async/task groups;
- ESP32/RTOS backend;
- Matrix2D, Canvas events/text/images, non-Windows presenters, and generic units/vectors.

The [quick reference](language-quick-reference.en.md) lists every public form and
builtin.
