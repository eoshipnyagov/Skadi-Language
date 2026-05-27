# Skadi Known Issues (Current Snapshot)

Date: 2026-05-27

## Fixed Recently

1. `Text` equality from `Text List` index could lower to pointer compare (`==`) instead of `strcmp`.
- Status: fixed in codegen.
- Guard: tests in `tests/codegen_smoke.rs`.

2. `output(keys[i])` for `Text List` index could route to integer output path.
- Status: fixed in codegen.
- Guard: tests in `tests/codegen_smoke.rs`.

3. Inline struct literal in `Struct List` push produced invalid C:
- Example: `sensors.push({id = 1, value = 17})`
- Old lowering: `sk_list_Sensor_push(&sensors, {.id = 1, ...})` (invalid C)
- Status: fixed via typed literal lowering `(Sensor){...}`.
- Guard: tests in `tests/codegen_smoke.rs`.

4. Runtime leak pressure from tracked text allocations (`concat/slice/input/read`) in long-running programs.
- Status: mitigated with runtime allocation tracking and cleanup.
- Guard: `scripts/leak_check_wsl.sh` + optional valgrind CI job.

## Open / Deferred

1. Nested list declarations are not supported by parser (`List(List(T))` style).
- Current behavior: parse error `SC-PARSE-140`.
- Contract is tested as negative case in `tests/codegen_e2e.rs`.

2. `Text List` C helper currently uses `const`-sensitive paths that may still produce compiler warnings in some environments.
- Current impact: warning noise, not functional failure in supported flows.
- Planned: tighten const-correctness end-to-end in list runtime helpers.

3. Top-level globals referenced from free functions in user programs can still expose codegen limitations depending on pattern.
- Current impact: some designs require inlining logic into `main`/loop scope.
- Planned: full global symbol model in codegen.
