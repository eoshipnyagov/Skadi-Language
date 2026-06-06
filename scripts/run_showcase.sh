#!/usr/bin/env bash

set -euo pipefail

MODE="${1:-all}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BENCHES=(
  "bench_01_tree|benchmarks/bench_01_tree.skd|benchmarks/showcase-data/tree_fixture|--dirs-only --depth-1"
  "bench_02_read_stats|benchmarks/bench_02_read_stats.skd|.|--input benchmarks/showcase-data/sample_weather.txt"
  "bench_03_find_count|benchmarks/bench_03_find_count.skd|.|--input benchmarks/showcase-data/sample_weather.txt --needle temperature"
  "bench_04_sum_ints|benchmarks/bench_04_sum_ints.skd|.|--small"
  "bench_05_push_pop|benchmarks/bench_05_push_pop.skd|.|--small"
  "bench_06_struct_account|benchmarks/bench_06_struct_account.skd|.|"
  "bench_07_struct_list|benchmarks/bench_07_struct_list.skd|.|"
  "bench_08_path_list_helpers|benchmarks/bench_08_path_list_helpers.skd|benchmarks/showcase-data/tree_fixture|"
  "bench_09_math_navigation|benchmarks/bench_09_math_navigation.skd|.|"
  "bench_10_v1_1_toolbox|benchmarks/bench_10_v1_1_toolbox.skd|.|"
)

build_bench() {
  local name="$1"
  local file="$2"
  local exe="$ROOT/$name.exe"
  echo "==> Build $name"
  rm -f "$exe"
  cargo run -- --input "$file" --emit-exe "$exe"
  [[ -f "$exe" ]] || {
    echo "Expected executable was not produced: $exe" >&2
    return 1
  }
}

smoke_bench() {
  local name="$1"
  local file="$2"
  local cwd="$3"
  shift 3
  local exe="$ROOT/$name.exe"
  [[ -f "$exe" ]] || build_bench "$name" "$file"
  echo "==> Run $name"
  (
    cd "$ROOT/$cwd"
    "$exe" "$@"
  )
}

for bench in "${BENCHES[@]}"; do
  IFS='|' read -r name file cwd args <<<"$bench"
  if [[ "$MODE" == "build" || "$MODE" == "all" ]]; then
    build_bench "$name" "$file"
  fi
  if [[ "$MODE" == "smoke" || "$MODE" == "all" ]]; then
    if [[ -n "$args" ]]; then
      # shellcheck disable=SC2206
      run_args=($args)
    else
      run_args=()
    fi
    smoke_bench "$name" "$file" "$cwd" "${run_args[@]}"
  fi
done

echo
echo "Showcase run completed successfully."
