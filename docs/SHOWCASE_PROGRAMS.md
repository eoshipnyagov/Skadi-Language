# Skadi Showcase Programs (Current v1 Prototype)

This folder contains 8 small showcase utilities.
Goal: verify that different real-world program shapes compile and run through the current Skadi -> C -> EXE pipeline.

## Programs

1. `bench_01_tree.skd`
- Recursive directory traversal.
- Coverage: recursion, `fs.list`, `fs.join`, `fs.is_dir`, flags parsing via `when`.

2. `bench_02_read_stats.skd`
- Reads file and prints chars/lines.
- Coverage: file I/O (`read`), text processing loops, `slice` + `find`.

3. `bench_03_find_count.skd`
- Counts substring occurrences in file content.
- Coverage: string scanning logic, control flow, builtins composition.

4. `bench_04_sum_ints.skd`
- Builds integer list and computes sum.
- Coverage: list construction, `push`, list iteration (`for in`), arithmetic.

5. `bench_05_push_pop.skd`
- Pushes and pops stack-like list.
- Coverage: list mutation, `pop() on error`, loop control.

6. `bench_06_struct_account.skd`
- Minimal account simulation with methods.
- Coverage: `struct`, `my.field`, method calls (`obj.method(...)`), typed struct literal.

7. `bench_07_struct_list.skd`
- Struct list traversal with method checks.
- Coverage: `Struct List`, `push` of struct literals, `iterate ... as ...`, method calls on list item.

8. `bench_08_path_list_helpers.skd`
- Path-oriented listing utility.
- Coverage: `Path List`, `fs.list`, `fs.join`, `fs.is_dir`, `iterate ... as ...`.

## Build all to EXE

From repo root:

```powershell
cargo run -- --input benchmarks/bench_01_tree.skd --emit-exe bench_01_tree.exe
cargo run -- --input benchmarks/bench_02_read_stats.skd --emit-exe bench_02_read_stats.exe
cargo run -- --input benchmarks/bench_03_find_count.skd --emit-exe bench_03_find_count.exe
cargo run -- --input benchmarks/bench_04_sum_ints.skd --emit-exe bench_04_sum_ints.exe
cargo run -- --input benchmarks/bench_05_push_pop.skd --emit-exe bench_05_push_pop.exe
cargo run -- --input benchmarks/bench_06_struct_account.skd --emit-exe bench_06_struct_account.exe
cargo run -- --input benchmarks/bench_07_struct_list.skd --emit-exe bench_07_struct_list.exe
cargo run -- --input benchmarks/bench_08_path_list_helpers.skd --emit-exe bench_08_path_list_helpers.exe
```

Or one-command helper:

```powershell
.\scripts\run_showcase.ps1 -Mode build
```

## Smoke runs

```powershell
.\bench_01_tree.exe --dirs-only --depth-3
.\bench_02_read_stats.exe --input example_meteostation.txt
.\bench_03_find_count.exe --input example_meteostation.txt --needle temperature
.\bench_04_sum_ints.exe --medium
.\bench_05_push_pop.exe --medium
.\bench_06_struct_account.exe
.\bench_07_struct_list.exe
.\bench_08_path_list_helpers.exe
```

Or one-command helper:

```powershell
.\scripts\run_showcase.ps1 -Mode smoke
```

Build and smoke in one pass:

```powershell
.\scripts\run_showcase.ps1 -Mode all
```

Notes:
- `run_showcase.ps1` validates that each expected `.exe` is produced.
- If compiler invocation fails, the script exits non-zero and reports failed benches.

## Why this set

- It covers different syntax and runtime paths, not just one demo.
- It is small enough to run frequently after compiler changes.
- It can serve as a style showcase for Skadi code.


