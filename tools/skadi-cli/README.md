# skadi-cli

Cargo-like CLI for Skadi (early stage).

## Current status

- Implemented:
  - `new`, `init`
  - `check` (real frontend pipeline: lex/parse/semantic)
  - `clean` (remove build artifacts; `--all` for deep clean)
  - `build` (Skadi -> C -> host exe via gcc/clang)
  - `run` (build + execute)
  - `target list`, `tui` (minimal)
  - `doctor` (target compiler availability report)
- Planned:
  - full target toolchain support
  - `format`

## Usage examples

```powershell
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- help
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- check
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- clean
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- clean --all
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- build
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- run
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor
```

