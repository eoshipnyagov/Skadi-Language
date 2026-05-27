# Skadi Language (Skadi -> C Compiler Prototype)

![CI](https://github.com/eoshipnyagov/Skadi-Language/actions/workflows/ci.yml/badge.svg)
![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)

Skadi is a systems language focused on calm readability: familiar control flow, less punctuation noise, and explicit behavior.
This repository contains the current `v1` compiler prototype and project manager.

Current pipeline:

`Skadi source -> lexer -> parser -> semantic -> C transpiler -> native C compiler`

## What Skadi Optimizes For

- code you can read without visual overload,
- explicit control flow and diagnostics,
- practical portability through the `Skadi -> C` backend.

## Quick Start

Use the installer scripts (recommended):

- Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install_skadi.ps1
```

- Linux/macOS/WSL:

```bash
bash ./scripts/install_skadi.sh
```

Then:

```bash
skadi doctor
skadi new console demo
cd demo
skadi check
skadi build
skadi run
```

If you run CLI through Cargo, `--` separates Cargo args from `skadi-cli` args.
Example: `cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor`.

## CLI Commands (Short)

- `skadi doctor`
- `skadi new <type> <name>`
- `skadi init [type]`
- `skadi examples`
- `skadi check`
- `skadi build`
- `skadi run`
- `skadi clean --all`
- `skadi tui`

Available `type` values for `skadi new <type> <name>`:

- `console`
- `game`
- `embedded`
- `gui`

More details: [docs/CLI_USAGE.md](docs/CLI_USAGE.md)

## Syntax Contrast (Tiny Example)

Task: sum positive values.

<details>
<summary>Skadi</summary>

```skadi
fn sum_positive(i32 List xs) i32 {
    new i32 total = 0
    iterate xs as x {
        if x > 0 {
            total += x
        }
    }
    return total
}
```
</details>

<details>
<summary>Rust</summary>

```rust
fn sum_positive(xs: &[i32]) -> i32 {
    let mut total = 0;
    for &x in xs {
        if x > 0 {
            total += x;
        }
    }
    total
}
```
</details>

<details>
<summary>Go</summary>

```go
func sumPositive(xs []int) int {
    total := 0
    for _, x := range xs {
        if x > 0 {
            total += x
        }
    }
    return total
}
```
</details>

<details>
<summary>Zig</summary>

```zig
fn sumPositive(xs: []const i32) i32 {
    var total: i32 = 0;
    for (xs) |x| {
        if (x > 0) {
            total += x;
        }
    }
    return total;
}
```
</details>

<details>
<summary>C++</summary>

```cpp
int sum_positive(const std::vector<int>& xs) {
    int total = 0;
    for (int x : xs) {
        if (x > 0) {
            total += x;
        }
    }
    return total;
}
```
</details>

## Repository Layout

- `src/` - compiler core (`lexer`, `parser`, `semantic_analysis`, `codegen`)
- `tools/skadi-cli/` - project manager CLI
- `tests/` - unit/integration/e2e tests
- `docs/` - specs, contracts, references, status matrices
- `benchmarks/` - showcase programs
- `examples/` - sample input/source files

## Key Docs

- [docs/QUICK_START.md](docs/QUICK_START.md)
- [docs/INSTALL_NEW_MACHINE.md](docs/INSTALL_NEW_MACHINE.md)
- [docs/SKADI_LANGUAGE_REFERENCE_RU.md](docs/SKADI_LANGUAGE_REFERENCE_RU.md)
- [docs/SKADI_SYNTAX_STATUS.md](docs/SKADI_SYNTAX_STATUS.md)
- [docs/SKADI_PROJECT_TECH_REFERENCE_RU.md](docs/SKADI_PROJECT_TECH_REFERENCE_RU.md)
- [docs/TEST_COVERAGE_MATRIX.md](docs/TEST_COVERAGE_MATRIX.md)
- [docs/DOCS_INDEX.md](docs/DOCS_INDEX.md)

## License

This project is licensed under the [MIT License](LICENSE).
