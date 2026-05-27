# Skadi Quick Start (Windows + WSL + Linux + macOS)

Date: 2026-05-27

## 1. Prerequisites

- Rust toolchain (`cargo`, `rustc`)
- Host C compiler in `PATH`:
  - Windows: `gcc` (MinGW) or `clang` or `cl`
  - Linux/WSL: `gcc` or `clang` or `cc`
  - macOS: `clang` (Xcode CLT)

## 2. Install `skadi` command

### Windows (PowerShell)

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install_skadi.ps1
```

### Linux / macOS / WSL

```bash
bash ./scripts/install_skadi.sh
```

Install scripts compile `skadi-cli`, create wrapper command `skadi`, and print PATH hints.

## 3. Verify toolchain

```bash
skadi help
skadi doctor
```

## 4. Create and run a project

```bash
skadi new console demo
cd demo
skadi check
skadi build
skadi run
skadi clean --all
```

## 5. Main manager commands

- `skadi doctor` - environment diagnostics
- `skadi new <type> <name>` - create project (`console|game|embedded|gui`)
- `skadi init [type]` - initialize in current dir
- `skadi examples` - add showcase examples
- `skadi check` - parse/semantic/codegen validation
- `skadi build` - build native binary
- `skadi run` - build and run
- `skadi clean --all` - remove generated artifacts
- `skadi tui` - interactive mode

Detailed command reference: `docs/CLI_USAGE.md`.

## 6. Notes

- Stable module contract in v1:
  - supported: `import "./relative_path.skd"`
  - not supported yet: `import module_name`, alias (`as`), visibility rules
- If you run via Cargo, `--` is required:
  - `cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor`
  - after `--`, args are passed to `skadi-cli`.
