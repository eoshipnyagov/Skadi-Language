# Skadi V1.1 Roadmap (Stability-First)

Date: 2026-05-27
Status: active execution plan

## Summary

V1.1 is a stabilization release.
Priority is deterministic compiler behavior, visibility/scope contract enforcement, and strict CI confidence.

Design baseline for visibility:
- struct fields are public by default, `hide` for hidden fields,
- functions are public by default, `local` for file/module-local declarations.

## Must (release-blocking)

1. Scope/Visibility contract implementation (`docs/internal/SCOPE_VISIBILITY_V1_1.md`)
- shadowing forbidden,
- `local fn/struct/label`,
- hidden field access only through methods of the same struct,
- direct-import-only visibility,
- deterministic import collision diagnostics,
- `module.symbol` qualification (`module` = filename without `.skd`) for `fn/struct/label`.

2. Diagnostics hardening
- keep stable `code + stage + hint` format for new semantic/import visibility failures.

3. Struct lowering stabilization
- fields, methods, `my` access, struct lists, and core nested cases.

4. Test/CI release gate
- full scope/visibility checklist in parser/semantic/codegen/e2e,
- required CI green on Win/Linux/macOS plus required `codegen-e2e` job.

## Should (target for 1.1, allowed in 1.1.x if needed)

1. Extend `on error` beyond `danger fn` and `List.pop` by explicit matrix.
2. Finalize `Text`/`List` edge contracts and diagnostics.
3. Stabilize `skadi build/run --cc <compiler>` as public contract.
4. Improve `doctor` with OS-specific actionable checks.

## Could (deferred to 1.2 by default)

1. `import module_name`, alias `as`, and re-export model.
2. `skadi docs` offline HTML and generated LLM guide.
3. Math/runtime expansion beyond current core (`Path/fs` reinforcement, math core, formatted output API).

## Definition of Done (V1.1)

1. All Must items closed by code + tests + docs updates.
2. No open P0 regressions in codegen/diagnostics/import graph behavior.
3. Required CI jobs are green on all three OS targets.
4. Docs are synchronized (`manual/internal`) and release RU snapshot process is followed per `docs/DOCS_POLICY.md`.
