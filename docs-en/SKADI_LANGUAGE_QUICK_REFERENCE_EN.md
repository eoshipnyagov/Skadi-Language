# Skadi Language Quick Reference

This is the compact inventory of the current public language surface.

- **Stable**: the `v1.1` base;
- **Experimental**: executable `v1.2` MVP with an unfrozen API;
- **Compatibility**: accepted for old source, not recommended;
- **Reserved**: held for a separate contract but not part of the compilable surface;
- **Future**: design only.

## Declarations and types

| Form | Example | Status |
|---|---|---:|
| Comments | `// line`, `/* block */` | Stable |
| Inferred declaration | `new count = 10` | Stable for scalar values |
| Typed declaration | `new Int count = 10` | Stable |
| List declaration | `new Int List ids = [1, 2]` | Stable |
| Assignment | `count = count + 1` | Stable |
| Compound assignment | `count += 1` | Compatibility; formatter expands it |
| Increment/decrement | `count++`, `count--` | Stable statements |
| Constant binding | `constant Int n = 1` | Stable |
| `;` and `:` | Punctuation tokens | Reserved outside legacy C-style `for` |

| Family | Types/literals | Status |
|---|---|---:|
| Integer | `Int`, `i8..i64`, `u8..u64`; `0b`, `0o`, `0x`, `_` | Stable |
| Floating point | `Float`/`f32` (32-bit), explicit `f64`, `0.5` | Stable |
| Boolean/character | `Bool`, `Char`, `true`, `'a'` | Stable |
| Text/path | `Text`, `Path`, `"hello"` | Stable |
| Collections/data | `Element List`, `struct Name` | Stable |
| Time | `Time`, `Duration`, `1ns`, `10us`, `5ms`, `2s`, `3min`, `1h` | Experimental |
| Memory size | `ByteSize`, `64b`, `4kb`, `8mb`, `1gb`, `1tb` | Experimental |
| Angle | `Angle`, `90deg`, `0.25rad` | Experimental |
| Vector | `Vec2`, `Vec3`, `Vec4` | Experimental |
| Runtime capabilities | `Memory`, `Task(T)`, `Channel(T)` | Experimental |

`bool` and `char` are compatibility aliases. `Char` is ASCII-only.

## Expressions and control flow

| Category | Forms |
|---|---|
| Arithmetic | `+ - * / % ^ div mod` |
| Comparison | `== != < > <= >=` |
| Logic | `and or xor not`; compatible `&& \|\| !`; single `&`/`\|` are lexer-only |
| Access | `value.field`, `my.field`, `items[index]` |
| Calls | `function(args)`, `value.method(args)` |
| Branching | `if/else if/else`, `when/is/else`; nominal cases allow `is Ready` |
| Loops | `iterate items as item`, `for item in items`, `while`, `loop` |
| Loop control | `break`, `continue`, `pass` |

C-style `for (init; condition; update)` is parse/format compatibility only and
is rejected by semantic analysis.

## Functions, data, modules, and errors

| Form | Example | Status |
|---|---|---:|
| Function | `fn add(Int a, Int b) returns Int { ... }` | Stable |
| Danger function | `danger fn load(Path path) returns Text { ... }` | Stable |
| Local symbol | `local fn/struct` | Stable |
| Local numeric label | `local label Code { Ok = 0 }` | Stable |
| Local symbolic tag | `local tag Status { Ready Busy }` | Stable |
| Return | `return value`, `return` | Stable |
| Error return | `return error Missing` | Stable in `danger fn` |
| Error label | `label ErrorCode { Ok = 0 Missing = 1 }` | Stable |
| Recovery | `value = danger_call() on error { ... }` | Stable |
| Read-only borrow | `fn inspect(view Canvas frame)`, `inspect(view frame)` | Experimental |
| Mutable borrow | `fn paint(direct Canvas frame)`, `paint(direct frame)` | Experimental |
| Ownership transfer | `fn consume(move Canvas frame)`, `consume(move frame)` | Experimental |
| Ownership return | `return move frame` | Experimental |
| Struct | `struct Point { Float x Float y }` | Stable |
| General `label` | `label ExitCode { Success = 0 Failed = 1 }` | Stable numeric nominal type |
| General `tag` | `tag Status { Ready Busy }` | Stable symbolic nominal type |
| Hidden field/self | `hide Int secret`, `my.secret` | Stable |
| Struct literal | `{x = 1.0, y = 2.0}`, `{x, y}` | Stable |
| Relative import | `import "./math.skd"` | Stable |
| Qualification | `math.add()`, `math.Point`, `math.Error` | Stable |
| Aliased path import | `import "./x.skd" as x` | Stable; alias is file-local |
| Package-name import | `import module` | Future |

The first `ErrorCode` variant must be `Ok`.

`view` passes a reference without permission to mutate through that parameter.
`direct` grants exclusive mutable access. Both borrows end with the synchronous
call and cannot cross a `run` boundary.

## Text, List, filesystem, and I/O

| Builtin | Signature | Result |
|---|---|---|
| `len` | `Text\|List` | `Int` |
| `contains` | `Text, Text` | `Bool` |
| `find` | `Text, Text` | `Int` |
| `slice` | `Text, Int, Int` | `Text` |
| `concat` | `Text, Text` | `Text` |
| `args` | none | `Text List` |
| `output` | one or more `Int\|Float\|Bool\|Char\|Text` values | `Int`; no implicit separators, one newline |
| `input` | `Text` | `Text` |
| `read` | `Text\|Path` | `Text` |
| `write` | `Text\|Path, Text` | `Int` |
| `fs.list` | `Text\|Path` | `Text List` |
| `fs.join` | `Text\|Path, Text` | `Text` |
| `fs.is_dir` | `Text\|Path` | `Bool` |

List operations are `items.push(value)` and
`value = items.pop() on error { ... }`.

## Math

Constants: `PI`, `TAU`, `E`, `EPSILON`.

| Builtins | Result |
|---|---|
| `abs`, `min`, `max`, `clamp` | Integer-preserving when all inputs are integers |
| `floor`, `ceil`, `round`, `sqrt`, `root` | `Float` |
| `sign` | Integer-preserving for `Int`, otherwise `Float` |
| `trunc`, `fract` | `Float` |
| `lerp`, `inverse_lerp`, `remap`, `smoothstep` | `Float` |
| `sin`, `cos`, `tan` | `Float`; require `Angle` |
| `asin`, `acos`, `atan`, `atan2`, `deg_to_rad` | `Angle` |
| `normalize_angle` | `Angle` in `[-PI, PI)` |
| `is_nan`, `is_finite`, `is_infinite` | `Bool` |
| `rad_to_deg`, `as_radians` | `Float` |
| `dot`, `length`, `length_sq`, `distance`, `distance_sq` | `Float` |
| `normalize` | Same vector type |
| `cross` | `Vec3` |

## Bit operations

`bit_and`, `bit_or`, `bit_xor`, `bit_not`, `bit_shift_left`,
`bit_shift_right`, `bit_is_set`, `bit_set`, `bit_clear`, `bit_toggle`, and
`bit_write` accept only `i8..i64` and `u8..u64`. Binary operands have the same
type; a fitting literal adopts the typed operand's type. `Int` is rejected,
right shift is logical, and indexes are checked. See
[platform Int and bit operations](bits.en.md).

## Systems surface

| Form | Meaning | Status |
|---|---|---:|
| `now`, `elapsed`, `sleep`, `delay` | Monotonic time and blocking delay | Experimental |
| `as_nanoseconds`, `as_bytes` | Exact underlying `i64` magnitude | Experimental |
| `Memory m = memory(4kb)` | Fixed-capacity region | Experimental |
| `memory(4kb, allow grow)` | Segmented growing region | Experimental |
| `memory(..., allow drop)` | Policy marker without implicit reclamation | Experimental |
| `memory.child(1kb)` | Fixed child of the active parent region | Experimental |
| `memory.static(4kb)` | Root-only static buffer | Experimental |
| `place in m { ... } on error { ... }` | Region placement/recovery | Experimental |
| `m.clear()` | Clear a region | Experimental |
| `Task t = run worker()` | Spawn a native task | Experimental |
| `value = wait t` | Join and retrieve result | Experimental |
| `value = wait t for 250ms on error { ... }` | Timed wait; success consumes the handle, timeout preserves it in the handler | Experimental |
| `stop t`, `stopping` | Cooperative stop | Experimental |
| `Channel(Int) q = channel(8)` | Bounded channel | Experimental |
| `q.send(value)`, `q.receive()` | Blocking message passing | Experimental |
| `q.send(value) on error { ... }` | Handle close or task cancellation | Experimental |
| `value = q.receive() on error { ... }` | Handle drain or cancellation | Experimental |
| `q.try_send(value)` | Non-blocking send attempt returning `Bool` | Experimental |
| `q.close()` | Owner closes the stream; queued values remain readable | Experimental |
| `q.send_for(value, 250ms) on error { ... }` | Blocking send with a deadline | Experimental |
| `value = q.receive_for(250ms) on error { ... }` | Blocking receive with a deadline | Experimental |
| `timed_out` | Timed Channel/Task handler reason; contextual identifier | Experimental |

Standalone `on error { ... }`, automatic `allow drop` reclamation, channel
`select`, `try_receive`, async/await, Matrix2D, Canvas
events/text/images, non-Windows window presenters, and package imports are not
implemented.

See the [topic reference](language-reference.en.md) and
[syntax status](syntax-status.en.md).
