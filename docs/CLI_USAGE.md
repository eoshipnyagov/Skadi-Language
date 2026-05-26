# Skadi CLI Usage

Date: 2026-05-26

This document describes practical usage of the current CLI manager in `tools/skadi-cli`.

## Commands

- `doctor` — detect host toolchain and show setup hints.
- `new <name> [--template <console|gui|embedded|game>]` — create a new project.
- `init` — initialize `skadi.toml` in current directory.
- `examples [--template <...>]` — add example programs.
- `check [--project <dir>]` — parse/semantic/codegen check without final native run.
- `build [--project <dir>] [--cc <compiler>] [--target <triple>]` — build project.
- `run [--project <dir>] [--cc <compiler>] [--target <triple>]` — build and run.
- `clean [--project <dir>]` — remove generated artifacts.

## Quick flow

```bash
cargo run -p skadi-cli -- doctor
cargo run -p skadi-cli -- new demo
cargo run -p skadi-cli -- check --project demo
cargo run -p skadi-cli -- build --project demo
cargo run -p skadi-cli -- run --project demo
```

## Notes on `cargo run -- ...`

When launching a binary via Cargo and passing flags to that binary, use:

```bash
cargo run -p skadi-cli -- <cli args>
```

The `--` separator tells Cargo that following arguments belong to `skadi-cli`, not Cargo itself.

## Compiler selection

`build` and `run` support explicit compiler pinning:

```bash
cargo run -p skadi-cli -- build --project demo --cc gcc
cargo run -p skadi-cli -- run --project demo --cc clang
```

Auto-detect order by host:
- Windows: `gcc -> clang -> cl`
- Linux/WSL/macOS: `gcc -> clang -> cc`

## Multi-file modules (v1 contract)

Supported canonical import form:

```skadi
import "./relative_path.skd"
```

Current v1 scope does not include alias/module-name import forms.
