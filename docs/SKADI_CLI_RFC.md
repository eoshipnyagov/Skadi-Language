# RFC: Skadi CLI (`skadi`) v0.1

Date: 2026-05-25
Status: Draft

## Goal

Create a Cargo-like tool for Skadi projects with:
- project initialization,
- build/check/run flows,
- target compilation,
- optional interactive mode via `skadi tui`.

## Command surface (v0.1)

- `skadi new <name>`: create project directory.
- `skadi init`: initialize project in current folder.
- `skadi check`: frontend checks only (lex/parse/semantic).
- `skadi build [--target <triple>]`: compile project.
- `skadi run`: build and run.
- `skadi target list`: show supported targets.
- `skadi tui`: interactive mode.
- `skadi format`: source formatting command.
- `skadi doctor`: environment checks.

## Project manifest (`Skadi.toml`)

```toml
[package]
name = "example"
version = "0.1.0"
edition = "v1"

[build]
entry = "src/main.scadi"
```

## Repo layout

- `tools/skadi-cli/`: CLI crate.
- main compiler crate remains in repo root.

## Near-term implementation order

1. `new/init` complete. ✅
2. `check` calls compiler frontend. ✅
3. `build` calls transpiler and system C compiler (host). ✅
4. `run` wraps build+exec. ✅
5. `target list` and `--target` mapping (host-only compile for now). ◑
6. `tui` expanded from minimal wizard to full flow. ◑
7. `format`, `doctor` production behavior. ◑

Current built-in target profiles:
- `host`
- `x86_64-w64-mingw32`
- `x86_64-unknown-linux-gnu`
