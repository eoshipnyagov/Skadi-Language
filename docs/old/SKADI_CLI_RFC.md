# RFC: Skadi CLI (`skadi`) v0.1

Date: 2026-05-26
Status: Draft

## Goal

Create a Cargo-like tool for Skadi projects with:
- project initialization,
- build/check/run flows,
- target compilation profiles,
- optional interactive mode via `skadi tui`.

## Command surface (v0.1)

- `skadi new <name>`: create project directory.
- `skadi new <type> <name>`: create typed project (`game|embedded|console|gui`).
- `skadi init [type]`: initialize project in current folder.
- `skadi examples`: add showcase examples for project type.
- `skadi check`: frontend checks + transpile check.
- `skadi clean [--all]`: remove generated build artifacts.
- `skadi build [--target <triple>] [--cc <compiler>]`: compile project.
- `skadi run [--target <triple>] [--cc <compiler>]`: build and run.
- `skadi target list`: show supported targets.
- `skadi tui`: interactive mode.
- `skadi format`: formatter entrypoint (planned behavior).
- `skadi doctor`: environment checks and compiler candidates.

## Project manifest (`skadi.toml`)

```toml
[package]
name = "example"
version = "0.1.0"
edition = "v1"
type = "console"

[build]
entry = "src/main.skd"
```

## Repo layout

- `tools/skadi-cli/`: CLI crate.
- compiler core crate remains in repository root.

## Implementation status snapshot (2026-05-26)

1. `new/init` complete. [done]
2. `check` frontend pipeline wired. [done]
3. `build` transpile + host C compiler. [done]
4. `run` build+exec wrapper. [done]
5. `target list` and target profile mapping. [done]
6. `tui` minimal interactive mode. [partial]
7. `format` placeholder command, `doctor` implemented. [partial]

Current built-in target profiles:
- `host`
- `x86_64-w64-mingw32`
- `x86_64-unknown-linux-gnu`

