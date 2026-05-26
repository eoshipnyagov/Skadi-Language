# Skadi Quick Start (Win + macOS + Linux)

Date: 2026-05-26

## 1. Prerequisites

- Rust toolchain (`cargo`, `rustc`)
- One host C compiler in `PATH`:
  - Windows: `gcc` (MinGW) or `clang` or `cl`
  - Linux/WSL: `gcc` or `clang` or `cc`
  - macOS: `clang` (Xcode CLT)

## 2. Install CLI command `skadi`

### Windows (PowerShell)

```powershell
cd D:\YandexDisk\Scadi\v01
powershell -ExecutionPolicy Bypass -File .\scripts\install_skadi.ps1
```

### Linux / macOS / WSL

```bash
cd /path/to/Scadi/v01
bash ./scripts/install_skadi.sh
```

If your shell cannot find `skadi` right after install, reopen terminal or update `PATH` as script suggests.

## 3. Verify environment

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
skadi examples
skadi clean
```

Typed templates:

```bash
skadi new game my_game
skadi new embedded firmware_demo
skadi new gui app_demo
```

## 5. Notes

- Current stable module contract in this wave:
  - supported: `import "./relative_path.skd"`
  - not supported yet: `import module_name`, alias (`as`), visibility rules
- If you run commands through `cargo run`, keep `--` before CLI args:
  - `cargo run --manifest-path tools/skadi-cli/Cargo.toml -- help`
