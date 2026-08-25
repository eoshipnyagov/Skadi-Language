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

The package resolver is not implemented yet. A bounded scalar C ABI is already
available through bodyless `external fn` declarations and `[native]` in
`Skadi.toml`; see [C ABI and Native C](c-abi.en.md). Packages will use the
manifest for local/Git dependencies plus a reproducible lock file, preserve
explicit direct imports, and resolve diamond graphs once.

The implemented slice covers fixed-width scalar functions and manifest linker
configuration. Explicit struct layouts, opaque owned/borrowed handles,
length-carrying buffers, headers, callbacks, and visible resource ownership are
future slices. Binding generation from simple headers must reuse this ABI
contract rather than define a second FFI model.
