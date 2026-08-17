# Files, Imports, and Visibility

```skadi
import "./math.skd"

new Int result = math.add(2, 3)
new math.Point origin = {x = 0.0, y = 0.0}
```

The qualifier is the file name without `.skd`. Imports may use a local alias:

```skadi
import "./long_player_model.skd" as player
```

A file sees public symbols only from modules it imports directly. Transitive
imports are not implicit re-exports. Diamond imports load one canonical module
once.

Top-level functions, structs, labels, and tags are public by default. Prefix
them with `local` to keep them in the file.

Named package imports, re-exports, and dependency resolution are not implemented.

## Accepted direction: packages and C libraries

Package and C-library integration are required on the path to a complete
language, but their syntax is not implemented yet. Packages will use
`Skadi.toml` for local/Git dependencies plus a reproducible lock file, preserve
explicit direct imports, and resolve diamond graphs once.

The first C ABI slice must cover fixed-width numbers, explicitly laid-out
structs, opaque owned/borrowed handles, length-carrying buffers, and linker
configuration. Ownership of `char*`, buffers, callbacks, and handles must be
visible. Binding generation from simple headers will use the same ABI contract;
it will not define a second FFI model.
