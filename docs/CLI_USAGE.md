# Skadi CLI Usage

Quick start (RU): `docs/SKADI_CLI_QUICK_START_RU.md`

Current CLI entrypoint: `src/main.rs`

## Supported commands

- Emit C file:
  - `cargo run -- --input program.skd --emit-c out.c`

- Build executable in one command (requires `gcc` or `clang` in PATH):
  - `cargo run -- --input program.skd --emit-exe out.exe`

- Print generated C to stdout:
  - `cargo run -- --input program.skd --print-c`

- Show help:
  - `cargo run -- --help`

## Notes

- `--emit-exe` uses a temporary `.c` file near output exe and removes it after compilation.
- If both `--emit-c` and `--emit-exe` are provided, compiler writes C output and also builds executable.
- You can pass source file directly without `--input`:
  - `cargo run -- program.skd --emit-exe out.exe`

## Planned next improvements

- Add explicit `--cc <compiler>` option (`gcc`, `clang`, `cl`).
- Add `--target <triple>` workflow for cross-compilation.
- Add stable `Skadi` wrapper binary command format.


