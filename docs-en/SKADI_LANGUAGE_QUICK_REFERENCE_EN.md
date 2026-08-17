# Skadi Language Quick Reference

This is the compact inventory of the current public language surface.

- **Stable**: the `v1.1` base;
- **Experimental**: executable `v1.2` MVP with an unfrozen API;
- **Compatibility**: accepted for old source, not recommended;
- **Reserved**: recognized by part of the frontend but not compilable;
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
| `fixed` / `const` | `fixed Int n = 1` | Reserved |
| `;` and `:` | Punctuation tokens | Reserved outside legacy C-style `for` |

| Family | Types/literals | Status |
|---|---|---:|
| Integer | `Int`, `i8..i64`, `u8..u64`, `42` | Stable |
| Floating point | `Float`, `f32`, `f64`, `0.5` | Stable |
| Boolean/character | `Bool`, `Char`, `true`, `'a'` | Stable |
| Text/path | `Text`, `Path`, `"hello"` | Stable |
| Collections/data | `Element List`, `struct Name` | Stable |
| Time | `Time`, `Duration`, `5ms`, `2s`, `3min` | Experimental |
| Memory size | `ByteSize`, `64b`, `4kb`, `8mb`, `1gb` | Experimental |
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
| Branching | `if/else if/else`, `when/is/else` |
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
| Local general label | `local label Status { Ready Busy }` | Partial |
| Return | `return value`, `return` | Stable |
| Error return | `return error Missing` | Stable in `danger fn` |
| Error label | `label ErrorCode { Ok Missing }` | Stable |
| Recovery | `value = danger_call() on error { ... }` | Stable |
| Read-only borrow | `fn inspect(view Canvas frame)`, `inspect(view frame)` | Experimental |
| Mutable borrow | `fn paint(direct Canvas frame)`, `paint(direct frame)` | Experimental |
| Ownership transfer | `fn consume(move Canvas frame)`, `consume(move frame)` | Experimental |
| Ownership return | `return move frame` | Experimental |
| Struct | `struct Point { Float x Float y }` | Stable |
| General `label` | `label Status { Ready Busy }` | Partial: declaration/visibility only |
| Hidden field/self | `hide Int secret`, `my.secret` | Stable |
| Struct literal | `{x = 1.0, y = 2.0}`, `{x, y}` | Stable |
| Relative import | `import "./math.skd"` | Stable |
| Qualification | `math.add()`, `math.Point`, `math.Error` | Stable |
| Named/aliased import | `import module`, `import "./x" as x` | Future |

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
| `output` | scalar/Text | `Int` |
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
| `sin`, `cos` | `Float`; prefer `Angle` input |
| `atan2`, `deg_to_rad` | `Angle` |
| `rad_to_deg` | `Float` |
| `dot`, `length`, `length_sq`, `distance`, `distance_sq` | `Float` |
| `normalize` | Same vector type |
| `cross` | `Vec3` |

## Systems surface

| Form | Meaning | Status |
|---|---|---:|
| `now`, `elapsed`, `sleep`, `delay` | Monotonic time and blocking delay | Experimental |
| `Memory m = memory(4kb)` | Fixed-capacity region | Experimental |
| `memory(4kb, allow grow)` | Segmented growing region | Experimental |
| `memory(..., allow drop)` | Policy marker without implicit reclamation | Experimental |
| `memory.child(1kb)` | Fixed child of the active parent region | Experimental |
| `memory.static(4kb)` | Root-only static buffer | Experimental |
| `place in m { ... } on error { ... }` | Region placement/recovery | Experimental |
| `m.clear()` | Clear a region | Experimental |
| `Task t = run worker()` | Spawn a native task | Experimental |
| `value = wait t` | Join and retrieve result | Experimental |
| `stop t`, `stopping` | Cooperative stop | Experimental |
| `Channel(Int) q = channel(8)` | Bounded channel | Experimental |
| `q.send(value)`, `q.receive()` | Blocking message passing | Experimental |
| `q.send(value) on error { ... }` | Handle close or task cancellation | Experimental |
| `value = q.receive() on error { ... }` | Handle drain or cancellation | Experimental |
| `q.try_send(value)` | Non-blocking send attempt returning `Bool` | Experimental |
| `q.close()` | Owner closes the stream; queued values remain readable | Experimental |
| `q.send_for(value, 250ms) on error { ... }` | Blocking send with a deadline | Experimental |
| `value = q.receive_for(250ms) on error { ... }` | Blocking receive with a deadline | Experimental |
| `timed_out` | Timed Channel handler reason; contextual identifier | Experimental |

Standalone `on error { ... }`, automatic `allow drop` reclamation, channel
`select`, timed `Task.wait`, `try_receive`, async/await, Matrix2D, Canvas
events/text/images, non-Windows window presenters, and package imports are not
implemented.

See the [topic reference](language-reference.en.md) and
[syntax status](syntax-status.en.md).
