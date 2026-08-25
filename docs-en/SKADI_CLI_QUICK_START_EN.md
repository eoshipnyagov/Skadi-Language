# CLI Quick Start

Run one file without creating a project:

```powershell
skadi-cli quick-run hello.skd
```

This needs a host C compiler, but does not need Cargo or `Skadi.toml` after
`skadi-cli` is installed.

For a regular project:

```powershell
skadi-cli doctor
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
```

Use `skadi-cli init` for an existing directory and `skadi-cli tui` for the
interactive workflow. Use `format --check` in CI.

For a sibling local Skadi package, add a relative path to the project manifest:

```toml
[dependencies]
physics = "../physics"
```

The package root must contain its own `Skadi.toml`; import a module with
`import "physics/src/vector.skd"`.

See the [complete CLI/TUI reference](cli-reference.en.md) for targets, compiler
selection, diagnostics, TUI keys, and planned commands.
