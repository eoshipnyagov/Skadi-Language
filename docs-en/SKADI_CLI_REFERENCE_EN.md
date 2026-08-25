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

For one-file experiments without a manifest:

```powershell
skadi-cli quick-run hello.skd
skadi-cli quick-run hello.skd -- first second
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
| `quick-run <file.skd> [-- <args ...>]` | Build and run one file without a manifest | Yes |
| `doctor` | Inspect host and cross toolchains | No |
| `target list` | List target profiles | No |
| `tui` | Open the full-screen project workflow | Per action |
| `--version`, `--help` | Identity and built-in help | No |

## Manifest

The ordinary `Int` width is configured in `skadi.toml` and is shared by CLI and
TUI builds:

```toml
[numeric]
int = "target" # target, i8, i16, i32, or i64
```

Current desktop targets resolve `target` to `i32`. Use explicit fixed-width
types for ABI, FFI, registers, and data formats.

```toml
[package]
name = "project"
version = "0.1.0"
edition = "v1"

[build]
entry = "src/main.skd"

[numeric]
int = "target"

[dependencies]
physics = "../physics"

[native]
sources = []
libraries = []
library_paths = []
```

Supported fields include `name`, `version`, `edition`, `entry`, `[numeric] int`,
local `[dependencies]` paths, and the `[native]` arrays. The TUI Config view
edits project/numeric fields and preserves dependency/native settings; a visual
dependency editor is not implemented yet.

A dependency path is relative to the current manifest and must point to a
directory with its own `Skadi.toml`. It enables imports such as
`import "physics/src/vector.skd"`. Git/registry dependencies and a lock file are
not implemented yet.

`[native]` configures the bounded [C ABI](c-abi.en.md): `sources` lists relative
`.c` files, `libraries` contains library names, and `library_paths` contains
relative search directories. Arbitrary compiler flags are not accepted.

## Build and run

```powershell
skadi-cli build --target host --cc clang
skadi-cli run --target host --cc gcc
```

Failures identify their source: Skadi frontend, project configuration, C
toolchain, runtime execution, or I/O. Build artifacts are written to `build/`.
Native source/path validation is reported as project configuration; C compiler
and linker failures are reported as toolchain errors.

## Quick run

`quick-run` is the short path for examples and small independent programs:

```powershell
skadi-cli quick-run examples/hello.skd
skadi-cli quick-run tools/inspect.skd -- input.txt --verbose
```

It does not look for `Skadi.toml`. Imports still resolve relative to the source
file. The generated C and executable live in a temporary directory and are
removed after execution. The command uses the host target, accepts `--cc`, and
forwards arguments only after `--`. Cargo is not required by an installed CLI.

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

After a successful `check` or `build`, the compiler core supplies structured
analysis facts to the TUI. The current workbench shows blocking and timed Channel
operations, path-sensitive timed Task wait, task-entry and `on error` context, incomplete `when`, and source-order
lifecycle chains for ownership, resources, Task, and Memory. These are
explanatory facts, not a second class of hard compiler errors.

## Planned tooling

- background TUI actions;
- richer per-command help;
- package/dependency commands after a module/package design exists;
- embedded build/flash after a platform runtime is approved.
