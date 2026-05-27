# Skadi CLI Usage

Date: 2026-05-27

This document describes practical usage of the current CLI manager in `tools/skadi-cli`.

## Commands

- `doctor` — detect host toolchain and show setup hints.
- `new <name>` or `new <type> <name>` — create a new project.
- `init [type]` — initialize `Skadi.toml` in current directory.
- `examples` — add example programs.
- `check` — parse/semantic/codegen check without final native run.
- `build [--target <triple>]` — build project.
- `run` — build and run.
- `clean [--all]` — remove generated artifacts.

Supported project types:

- `console`
- `game`
- `embedded`
- `gui`

Examples:

- `skadi new demo` (same as `skadi new console demo`)
- `skadi new game my_game`
- `skadi new embedded sensor_fw`
- `skadi new gui app`

## Quick flow

```bash
skadi doctor
skadi new console demo
cd demo
skadi check
skadi build
skadi run
```

## Notes on `cargo run -- ...`

If `skadi` is not installed in PATH yet and you run via Cargo:

```bash
cargo run -p skadi-cli -- <cli args>
```

The `--` separator tells Cargo that following arguments belong to `skadi-cli`, not Cargo itself.

## Compiler selection

Current stable command surface does not expose `--cc` yet.
Compiler auto-detect order is:

```bash
- Windows: `gcc -> clang -> cl`
- Linux/WSL/macOS: `gcc -> clang -> cc`

## Multi-file modules (v1 contract)

Supported canonical import form:

```skadi
import "./relative_path.skd"
```

Current v1 scope does not include alias/module-name import forms.


