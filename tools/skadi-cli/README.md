# skadi-cli

Cargo-like CLI for Skadi (early stage).

Quick start (RU): `docs/SKADI_CLI_QUICK_START_RU.md`

## Current status

- Implemented:
  - `new`, `init`
  - `check` (real frontend pipeline: lex/parse/semantic)
  - `build` (Skadi -> C -> host exe, supports `--target` and `--cc`)
  - `run` (build + execute, supports `--target` and `--cc`)
  - `target list`, `tui` (minimal)
  - `doctor` (target compiler availability report)
- Planned:
  - full target toolchain support
  - `format`

## Usage examples

```powershell
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- help
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- check
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- build
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- build --target host --cc gcc
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- run
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor
```

