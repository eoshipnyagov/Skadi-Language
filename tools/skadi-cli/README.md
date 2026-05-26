# skadi-cli

Cargo-like CLI for Skadi (early stage).

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
  - multi-file project load via `import "./relative_file.skd"` (recursive, cycle-safe, deduplicated)
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

