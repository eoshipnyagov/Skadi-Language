# Skadi: Setup on a New Machine

Date: 2026-05-27

## 1. Requirements

- Git
- Rust toolchain (`cargo`, `rustc`)
- C compiler in `PATH`:
  - Windows: `gcc` (MinGW) or `clang` or `cl`
  - Linux/WSL: `gcc` or `clang` or `cc`
  - macOS: `clang` (Xcode Command Line Tools)

## 2. Clone

```bash
git clone git@github.com:eoshipnyagov/Skadi-Language.git
cd Skadi-Language
```

## 3. Install `skadi` command

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install_skadi.ps1
```

Linux/macOS/WSL:

```bash
bash ./scripts/install_skadi.sh
```

## 4. Validate

```bash
skadi doctor
skadi new console demo
cd demo
skadi check
skadi build
skadi run
```

Expected result: `check/build/run` succeed and output contains `Hello from Skadi!`.

## 5. Cargo fallback

If `skadi` is not yet in `PATH`, run:

```bash
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor
```

`--` is required to separate Cargo args from `skadi-cli` args.

## 6. Troubleshooting

1. `gcc/clang not found`
- Install a supported C compiler.
- Verify with `gcc --version` or `clang --version`.
- Run `skadi doctor`.

2. `failed to read Skadi.toml`
- Run `check/build/run` from the project directory.
- `cd` into project root and retry.

3. `cargo not found`
- Install Rust via rustup.
- Restart terminal.

4. Build fails on a fresh OS
- Run:
  - `cargo test -q`
  - `cargo clippy --all-targets --all-features`
  - `cargo clippy --manifest-path tools/skadi-cli/Cargo.toml --all-targets --all-features`
- Compare with latest green GitHub Actions run.

## 7. Related docs

- `docs/QUICK_START.md`
- `docs/CLI_USAGE.md`
- `docs/DOCS_INDEX.md`
