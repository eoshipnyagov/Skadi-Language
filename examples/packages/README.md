# Local package example

`app` imports a module from the sibling `math-kit` Skadi package through a
local dependency declared in `app/Skadi.toml`.

```powershell
cd examples/packages/app
skadi-cli check
skadi-cli run
```

Expected output:

```text
Package answer: 42
```

The example intentionally uses only local paths. Git/registry dependencies,
version solving, and lock files are not part of the current package slice.
