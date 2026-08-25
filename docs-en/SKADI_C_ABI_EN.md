# C ABI and Native C

Skadi can call small C APIs through explicit external declarations. The current
MVP deliberately supports scalars and typed buffers. It is useful for trusted C
helpers, driver adapters, and testing the FFI design without exposing raw
pointers or hiding resource ownership.

## Minimal example

A runnable project is available in `examples/c-abi`; its declarations live in a
separate `src/sensor.skd` binding module. The shortened single-file version
below shows its essential parts.

`src/main.skd`:

```skadi
external fn c_add(i32 left, i32 right) returns i32

new i32 answer = c_add(20, 22)
output(answer)
```

`native/helper.c`:

```c
#include <stdint.h>

int32_t c_add(int32_t left, int32_t right) {
    return left + right;
}
```

`Skadi.toml`:

```toml
[native]
sources = ["native/helper.c"]
libraries = []
library_paths = []
```

Use the normal project workflow:

```powershell
skadi-cli check
skadi-cli run
```

`check` validates the Skadi declaration and manifest. `build`/`run` also verify
native paths, compile the C sources, and link the result.

## Declaration syntax

```skadi
external fn name(Type argument, Type other) returns ReturnType
external fn notify()
```

An `external fn` has no body. Omitting `returns` means C `void`. External calls are
typed like ordinary Skadi calls, but an external function cannot be started
directly with `run`; use a Skadi wrapper as the task entry point.

## Errors with `external danger fn`

An external C adapter can use the ordinary Skadi error model:

```skadi
external danger fn sensor_read(i32 channel) returns i32

new i32 reading = 0
reading = sensor_read(2) on error {
    output("sensor read failed")
}
```

The corresponding C ABI is:

```c
int sensor_read(int32_t channel, int32_t *out) {
    if (out == 0) {
        return 1;
    }
    *out = 400;
    return 0;
}
```

Zero means success; a non-zero status enters `on error`. With `returns T`, the
value is written through the final `T *out` parameter. Without `returns`, the C
function only returns its status. Semantic analysis rejects an unhandled danger
call used as an ordinary expression.

## Supported types

| Skadi | C |
|---|---|
| `i8`, `i16`, `i32`, `i64` | `int8_t`, `int16_t`, `int32_t`, `int64_t` |
| `u8`, `u16`, `u32`, `u64` | `uint8_t`, `uint16_t`, `uint32_t`, `uint64_t` |
| `f32` | `float` |
| `f64` | `double` |
| `Bool` | `bool` |
| `Char` | `char` |
| no `returns` | `void` |

For `external danger fn`, the C return type is always `int`, and the declared
Skadi result becomes the final out parameter.

`Int`, `Float`, `Text`, `Path`, lists, structs, specialized types, and owning
resources are not ABI types in this slice. Use fixed-width types at every C
boundary. Scalar parameters are passed by value, and `move` is not accepted at
the C boundary.

## Typed buffers

`Buffer(T)` is available only as an `external fn` parameter. It bridges a typed
Skadi `T List` to a conventional C pointer-and-length pair:

```skadi
external fn checksum(view Buffer(u8) data) returns u32
external danger fn adjust(edit Buffer(u8) data, u8 delta)

new u8 List packet = [10, 20, 30]
new u32 sum = checksum(view packet)
adjust(edit packet, 1) on error {
    output("adjust failed")
}
```

```c
uint32_t checksum(const uint8_t *data, size_t data_length);
int adjust(uint8_t *data, size_t data_length, uint8_t delta);
```

- `view Buffer(T)` lowers to read-only `const T *` access;
- `edit Buffer(T)` lowers to exclusive mutable `T *` access;
- length uses C `size_t` and counts elements rather than bytes;
- `T` must be a supported fixed scalar ABI type;
- `Buffer(T)` cannot be stored, returned, placed in a struct/list, or cross `run`;
- the borrow ends after the synchronous C call, so native code must not retain
  the pointer.

## `[native]` fields

| Field | Meaning |
|---|---|
| `sources` | Project C files; relative `.c` paths only |
| `libraries` | System/prebuilt library names, not shell flags |
| `library_paths` | Relative library search directories |

Paths are resolved from the project root, cannot be absolute, and cannot contain
`..`. GCC/Clang receive `-L...`/`-l...`; MSVC receives `/LIBPATH:...` and `.lib`.
Arbitrary compiler or linker flags are not accepted through the manifest.

The TUI Config editor preserves these fields, but the current MVP edits them
directly in `Skadi.toml`.

## Guidance and limits

- Keep the C boundary small and wrap domain behavior in Skadi functions.
- Keep the C signature and `external fn` declaration identical; Skadi cannot
  detect an ABI mismatch.
- Native sources and libraries must support the selected cross target.
- Do not encode C pointers as integers.
- Header import/generation, symbol aliases, calling conventions, C struct/enum
  layout, raw pointers, nullable values, callbacks, variadics, C++, and package
  resolution are not implemented yet.
