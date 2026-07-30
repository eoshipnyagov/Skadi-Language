# Skadi CLI and TUI Reference

`skadi-cli` is the primary toolchain for Skadi `v1.2.0-rc.1`.

## Essentials

```powershell
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
```

```powershell
skadi-cli doctor
skadi-cli tui
```

## Commands

| Command | Purpose | C compiler |
|---|---|---:|
| `new <name>` | Create a project directory, manifest, and entry | No |
| `init` | Initialize the current directory | No |
| `check` | Imports, lexer, parser, semantic checks | No |
| `format [--check] [path ...]` | Write or verify canonical formatting | No |
| `build [--target name] [--cc compiler]` | Build a native binary | Yes |
| `run [--target name] [--cc compiler]` | Build and execute | Yes |
| `doctor` | Inspect host and cross toolchains | No |
| `target list` | List target profiles | No |
| `tui` | Open the full-screen project workflow | Per action |
| `--version`, `--help` | Identity and built-in help | No |

## Manifest

```toml
[package]
name = "project"
version = "0.1.0"
edition = "v1"

[build]
entry = "src/main.skd"
```

Supported fields are `name`, `version`, `edition`, and `entry`. The TUI Config
view edits the same fields.

## Build and run

```powershell
skadi-cli build --target host --cc clang
skadi-cli run --target host --cc gcc
```

Failures identify their source: Skadi frontend, project configuration, C
toolchain, runtime execution, or I/O. Build artifacts are written to `build/`.

## TUI

Screens cover project status, diagnostics, build/run output, doctor, bootstrap,
manifest configuration, and help.

| Key | Action |
|---|---|
| `c`, `b`, `r`, `f`, `d` | check, build, run, format, doctor |
| `p`, `e`, `m`, `h` | project, diagnostics, config, help |
| `o`, `g` | open project, generate a missing entry |
| `Tab`, `Shift+Tab`, arrows, `j/k` | navigation |
| `Enter`, `q` | activate, quit |

Actions are currently synchronous. The TUI is not a source editor or showcase
browser. CLI commands remain canonical for scripts and CI.

## Planned tooling

- background TUI actions;
- richer per-command help;
- package/dependency commands after a module/package design exists;
- embedded build/flash after a platform runtime is approved.
