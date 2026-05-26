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

## Quick flow

```bash
cargo run -p skadi-cli -- doctor
cargo run -p skadi-cli -- new console demo
cd demo
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- check
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- build
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- run
```

## Notes on `cargo run -- ...`

When launching a binary via Cargo and passing flags to that binary, use:

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
