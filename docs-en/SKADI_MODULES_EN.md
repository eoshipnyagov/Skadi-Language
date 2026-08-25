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

Re-exports, remote dependency resolution, and standalone module declarations are
not implemented.

## Local package dependencies

Projects can name sibling or vendored Skadi packages in `Skadi.toml`:

```toml
[dependencies]
physics = "../physics"
ui = "vendor/ui"
```

Each path is relative to the current manifest. The dependency root must contain
its own `Skadi.toml`. Import a file through the dependency name:

```skadi
import "physics/src/vector.skd" as vectors

new vectors.Body body = vectors.make_body()
```

The first path component selects `[dependencies]`; the default qualifier still
comes from the imported file name, while `as` is local to the importing file.
`.` and `..` are forbidden inside a package path, and canonical resolution may
not escape the dependency root. Unknown packages and invalid roots report
`SC-MOD-004` within the module stage.

Direct visibility, canonical diamond deduplication, `local`, and collision rules
are identical for project and package modules. A package's dependency is not
implicitly visible to the application.

## Packages and C libraries

The first local-path package resolver is implemented. A bounded C ABI is already
available through bodyless `external fn` declarations and `[native]` in
`Skadi.toml`; see [C ABI and Native C](c-abi.en.md). Git/registry sources,
version constraints, transitive resolution, and a reproducible lock file remain
future work; the local resolver deliberately does not create a placeholder lock.

The implemented slice covers fixed-width scalar functions, by-value external
structs, call-scoped typed buffers, opaque owning `external resource` handles,
and manifest linker configuration. Custom layouts, borrowed external
lifetimes, long-lived buffers, headers, and callbacks remain future slices.
Binding generation from simple headers must reuse this ABI contract rather
than define a second FFI model.
