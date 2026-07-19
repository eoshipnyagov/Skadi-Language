# Skadi Scope & Visibility Contract v1.1

Date: 2026-07-19
Status: implemented and covered by parser, semantic, codegen, and CLI pipeline tests

## 1. Shadowing

- Shadowing is forbidden.
- Any `new <type> name = ...` that reuses an already-declared name from an outer scope is a compile error.

## 2. Local names vs struct fields

- `my.field` always resolves to a struct field.
- `field` (without `my.`) resolves to a local variable or parameter.
- Name collision between local and field is allowed by language rules, but style warning is recommended.

## 3. Default visibility

- Struct fields are public by default.
- Functions are public by default.
- Hide field explicitly with `hide`.
- Hide declaration in module/file with `local`.

## 4. `local` applicability

- `local fn` is supported.
- `local struct` is supported.
- `local label` is supported.

## 5. Hidden field access

- `hide` fields are accessible only through methods of the same struct.
- Direct external access is forbidden.

## 6. Import visibility model

- Only direct imports expose symbols.
- Transitive visibility is not supported in v1.1.
  - If `A` imports `B`, importing `A` does not expose symbols of `B`.

## 7. Import conflicts

- Name conflicts from multiple imports are compile errors.
- Conflict resolution uses qualified form: `module.symbol`.

## 8. Module qualification

- `module` is the source filename without `.skd`.
- Qualified access `module.symbol` is allowed for:
  - functions,
  - structs,
  - labels.

## 9. Declaration order

- Functions may be used before their textual declaration in a file.

## 10. Test checklist

1. Negative: shadowing in nested block fails.
2. Positive: `my.field` and local parameter with same name resolve correctly.
3. Positive/negative: `hide` field direct access fails, access via own method succeeds.
4. Positive: `local fn/struct/label` hidden from importer.
5. Negative: import name collision reports deterministic diagnostic.
6. Positive: `module.symbol` works for `fn`, `struct`, `label`.
7. Negative: transitive import visibility is rejected.

All checklist items are covered in the current test suites. See
[Test Coverage Matrix](test-coverage.md) for the maintained coverage overview.
