# Skadi Showcase Programs

This page collects 10 small showcase programs for the current Skadi toolchain.

They serve three roles:

- validate that real programs still pass through the current `Skadi -> C -> executable` path;
- act as a readable product-facing language showcase;
- provide a fast smoke set after compiler and runtime changes.

## Included programs

1. `bench_01_tree.skd`
   Recursive directory walk with `fs.list`, `fs.join`, `fs.is_dir`, and `when`.
2. `bench_02_read_stats.skd`
   Reads a file and prints basic text statistics.
3. `bench_03_find_count.skd`
   Counts substring matches in file content.
4. `bench_04_sum_ints.skd`
   Fills an integer list and computes a sum.
5. `bench_05_push_pop.skd`
   Uses a list as a stack with `pop() on error`.
6. `bench_06_struct_account.skd`
   Minimal struct-and-method example with `my.field` and dot-call syntax.
7. `bench_07_struct_list.skd`
   Iterates over a `Struct List` and calls methods on list elements.
8. `bench_08_path_list_helpers.skd`
   Small path-list utility with `fs.list`, `fs.join`, and `fs.is_dir`.
9. `bench_09_math_navigation.skd`
   Compact math/navigation showcase.
10. `bench_10_v1_1_toolbox.skd`
    Combined `v1.1` showcase for danger calls, lists, structs, `when`, and math.

## Stable showcase fixtures

The repository includes reproducible fixture data in `benchmarks/showcase-data/`:

- `sample_weather.txt` is used by `bench_02_read_stats.skd` and `bench_03_find_count.skd`;
- `tree_fixture/` is used by `bench_01_tree.skd` and `bench_08_path_list_helpers.skd`.

These fixtures are shared by smoke scripts and e2e showcase tests.

## Build and smoke scripts

Windows / PowerShell:

```powershell
.\scripts\run_showcase.ps1 -Mode build
.\scripts\run_showcase.ps1 -Mode smoke
.\scripts\run_showcase.ps1 -Mode all
```

POSIX shell:

```bash
./scripts/run_showcase.sh build
./scripts/run_showcase.sh smoke
./scripts/run_showcase.sh all
```

## Coverage notes

- compile-pipeline showcase tests cover `bench_01..10`;
- native build tests cover `bench_01..10`;
- runtime e2e coverage is split into:
  - CLI-driven showcase subset `bench_01..05`,
  - stable showcase subset `bench_06..09`,
  - dedicated `bench_10` showcase e2e.

Russian remains the primary source of truth for the full showcase documentation.
