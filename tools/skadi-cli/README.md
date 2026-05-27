# skadi-cli

Cargo-like CLI for Skadi (early stage).

Quick start (cross-platform): `docs/manual/QUICK_START.md`

## Current status

- Implemented:
  - `new`, `init`
    - `new <name>` (default `console`)
    - `new <type> <name>` where `type in {game, embedded, console, gui}`
  - `check` (real frontend pipeline: lex/parse/semantic)
  - `clean` (remove build artifacts; `--all` for deep clean)
  - `build` (Skadi -> C -> host exe via gcc/clang)
  - `run` (build + execute)
  - `examples` (inject typed example set into `examples/`)
  - `target list`, `tui` (minimal)
  - `doctor` (target compiler availability report)
  - multi-file project load via `import "./relative_file.skd"` (recursive, cycle-safe, deduplicated, deterministic)
- Planned:
  - full target toolchain support
  - `format`

## Usage examples

```powershell
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- help
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- check
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- clean
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- clean --all
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- new game my_game
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- examples
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- examples --type gui
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- build
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- run
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor
```

## Install as global `skadi` command

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install_skadi.ps1
```

Linux / macOS / WSL:

```bash
bash ./scripts/install_skadi.sh
```

## From-zero smoke flow (Windows PowerShell)

Run the full newcomer path in a temporary directory:

```powershell
.\scripts\smoke_from_zero.ps1 -Type console
```

Other project types:

```powershell
.\scripts\smoke_from_zero.ps1 -Type game
.\scripts\smoke_from_zero.ps1 -Type embedded
.\scripts\smoke_from_zero.ps1 -Type gui
```

## V1 module contract (current wave)

- Supported:
  - `import "./relative_path.skd"`
- Not supported in this wave:
  - `import module_name`
  - alias form (`import "./x.skd" as x`)
  - visibility rules (`local fn`/module privacy semantics)
- Import pipeline behavior:
  - recursive loading
  - cycle detection (fails fast)
  - deduplication of already imported files
  - deterministic merge order by import order in source



