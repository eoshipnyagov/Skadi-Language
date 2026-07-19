# Skadi Diagnostic Codes Reference (v1 Snapshot)

Date: 2026-07-19
Purpose: canonical map of diagnostic codes by pipeline stage and ownership.

## 1. Code Families

- `SC-LEX-*` — lexer diagnostics (`src/lexer/*`)
- `SC-PARSE-*` — parser diagnostics (`src/parser/*`)
- `SC-SEM-*` — semantic diagnostics (`src/semantic_analysis.rs`)
- `SC-MOD-*` — module/import pipeline diagnostics (`tools/skadi-cli/src/pipeline.rs`)
- `SC-CGEN-*` — native C compile/link diagnostics (`tools/skadi-cli/src/pipeline.rs`)

Wrapper/stage codes used by CLI pipeline:
- `SC-LEX-000` — lex stage wrapper in `compile_to_c`
- `SC-PARSE-000` — parse stage wrapper in `compile_to_c`
- `SC-SEM-000` — semantic stage wrapper in `compile_to_c`

## 2. Parser Codes (`SC-PARSE-*`)

### Entry / wrapper
- `SC-PARSE-001` unexpected token at statement start
- `SC-PARSE-002` failed to parse statement at token index
- `SC-PARSE-003` statement parser returned error wrapper

### Statement parser ranges
- `SC-PARSE-101..120` block/function/loop/when structure errors
- `SC-PARSE-121..128` if/while/loop/return structure errors
- `SC-PARSE-129..136` assignment and danger-call/on-error shape errors
- `SC-PARSE-137..148` declarations (`new/label/struct/on`) shape errors
- `SC-PARSE-149..152` `iterate ... as ...` alias syntax errors
- `SC-PARSE-153..157` struct-method signature/body shape errors
- `SC-PARSE-161..162` `local` prefix declaration contract errors

### Expression parser ranges
- `SC-PARSE-201..216` expression grammar errors
  - grouped expr / call args / list literal / index / struct literal issues
  - `SC-PARSE-216` invalid, fractional or overflowing Duration literal

## 3. Semantic Codes (`SC-SEM-*`)

- `SC-SEM-010` redeclaration in same scope
- `SC-SEM-011` invalid self-referential initialization
- `SC-SEM-012` use-before-definition
- `SC-SEM-020` type mismatch / invalid operand context
- `SC-SEM-030` unknown function
- `SC-SEM-031` argument count mismatch
- `SC-SEM-032` argument type mismatch
- `SC-SEM-033` builtin argument contract mismatch
- `SC-SEM-040` invalid semantic context
- `SC-SEM-050` return-path/return-form rule
- `SC-SEM-051` `ErrorCode` contract rule
- `SC-SEM-060` Memory usage and placement rule
- `SC-SEM-061` Memory lifetime and escape rule
- `SC-SEM-062` Memory capability/storage rule
- `SC-SEM-070` Task lifecycle and context rule
- `SC-SEM-071` Task capability/boundary rule
- `SC-SEM-080` Channel ownership/message rule
- `SC-SEM-900` internal semantic consistency error

### Runtime codes

- `SC-RT-301..304` Task allocation/start/join/stop synchronization failures
- `SC-RT-311..313` Channel allocation/capacity/synchronization failures
- `SC-RT-320` monotonic clock or blocking sleep runtime failure

## 4. Module / CLI Pipeline Codes

- `SC-MOD-001`
  - stage: `skadi-cli` import/merge pipeline
  - meaning: path-import contract violation or import graph failure
  - includes: unsupported module-name import, alias import, missing import file, cyclic import
- `SC-MOD-002`
  - stage: `skadi-cli` import/merge pipeline
  - meaning: deterministic public symbol collision across imported modules
  - includes: same public `fn/struct/label` name declared in multiple imported modules
- `SC-MOD-003`
  - stage: `skadi-cli` import/merge pipeline
  - meaning: direct-import-only visibility violation at entry file
  - includes: entry file calling symbol available only through transitive import chain

- `SC-CGEN-001`
  - stage: native C compiler invocation after transpilation
  - meaning: all compiler attempts failed
  - contract: message includes target and attempts matrix (`- <compiler>: <detail>`)

## 5. Output Format Contract

Canonical formatting is defined in [Diagnostics Style Guide](diagnostics-style.md):

`<Kind> error at line <L>, col <C>[, index <I>]: [<CODE>] <message>`

For module/codegen pipeline codes, location may be absent and message is still required to include code and actionable hint.

For CLI pipeline wrappers, normalized prefix is:
- `[<CODE>] stage=<stage-name>: <details>`
- followed by `hint: <actionable next step>`

## 6. Ownership and Change Policy

1. New diagnostic code must be added here and in stage-local tests.
2. Reusing an existing code for a different semantic meaning is not allowed.
3. Parser/semantic/module/codegen tests must assert code presence for stable failure contracts.
