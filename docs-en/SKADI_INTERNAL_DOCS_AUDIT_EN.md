# Internal Documentation Audit

Review date: 2026-07-30  
Baseline: `develop`, release candidate `v1.2.0-rc.1`

## Source-of-truth order

When documents disagree, use this order:

1. tests and actual `lexer -> parser -> semantic -> codegen -> native` behavior;
2. syntax status and language quick reference;
3. current MVP contracts;
4. accepted `v1` contracts;
5. historical release plans;
6. drafts and unaccepted RFCs.

A draft is design intent, not a syntax promise. A lexer token is not a language
feature by itself.

## Architecture understanding synchronized on 2026-07-30

- Skadi is an integrated official environment, not only parser grammar plus
  unrelated libraries.
- Its official layers are Language Core, Systems Core, Domain Core, and
  Platform Backends.
- Canvas remains a Domain Core teaching and prototyping surface, but it must not
  grow into a complete GUI, media framework, or game engine.
- The language should teach resource management. Scope, copy, `view`, `direct`,
  `move`, cleanup, Memory, and Task/Channel behavior must be visible through
  diagnostics and tooling.
- `when / is` is the strict Skadi `switch / case`: evaluate once, never fall
  through, reject duplicates, and eventually check exhaustiveness.
- TUI is a first-class client of a shared analysis/debug engine, not a second
  implementation of compiler rules.
- The first debugger is Skadi-level: source mapping, probes, breakpoints,
  locals, and runtime views over the C pipeline. A custom machine debugger is
  not planned.
- Experimental ideas may end as `Rejected`; compatibility begins only after a
  surface is promoted to stable/canonical.

## Current classification

| Area | Current state |
|---|---|
| Compiler/CLI/backend references | Living and synchronized with `v1.2.0-rc.1` |
| `v1` Text/List/error/style contracts | Accepted historical baseline |
| `v1.1` plans | Historical and completed |
| `v1.2` plan | Living release ledger |
| Memory and Task/Channel MVP contracts | Executable experimental runtime MVP |
| Time, ByteSize, Angle, Vector contracts | Executable experimental runtime MVP |
| Systems Additions | Partly implemented; resources/context/devices remain future |
| Visual Core / Canvas | Experimental Canvas v0 with headless renderer and Win32 presenter |
| CLI RFC v0.1 | Historical and superseded by the current reference |

## Main design/implementation gaps

### Memory and ownership

The implemented region slice supports fixed capacity by default, segmented
`allow grow`, declarative `allow drop`, child regions, root-only static
regions, `place in`, `clear`, dynamic-payload escape checks, and thread-local
active regions. `Canvas`, `Window`, `Interrupt`, and owning `Channel` handles
share explicit `move` transfer with branch/loop checks and deterministic
cleanup.

Automatic `allow drop` reclamation, an embedded allocator backend, first-class
references, and a general lifetime calculus are not implemented. `Memory`
remains a non-movable region capability and `Task` retains consume-through-
`wait`.

### Task and Channel

The runtime uses Win32/pthread native threads, linear owning handles, mandatory
`wait`, cooperative stop, and bounded FIFO channels with
`send/receive/try_send/close`, drain-after-close, cancellation-aware blocking
operations, Duration-bounded `send_for/receive_for`, and path-sensitive timed
Task wait. Scheduler abstraction, async/await, task groups, select, and RTOS
backends remain future work.

### Specialized types

Time/Duration, ByteSize, Angle, and Vec2/Vec3/Vec4 execute end-to-end but remain
experimental because their APIs are not frozen. Wall clock/calendar, unit
algebra, angle normalization, generic/SIMD vectors, and matrices are absent.

### Labels and tags

The gap is closed with two nominal forms. `label` requires an explicit numeric
discriminant for every variant, while `tag` defines symbolic variants without a
user-visible number. Both forms pass parser, semantic, formatter, module, and C
lowering; `ErrorCode` remains the special label that starts with `Ok = 0`.

### Modules and I/O

Relative path imports, direct-import-only visibility, and
`import "./x.skd" as alias` work. Named package imports, re-exports, and
dependency resolution do not. The current I/O model is deliberately small and
synchronous; an old stream idea is not an accepted roadmap commitment.

### Embedded and Canvas

The host MVP implements typed periodic `Interrupt` and a strict interrupt-safe
handler subset. ESP32 still needs a toolchain profile, RTOS/bare-metal runtime
adapters, linker/flash workflow, and validation. Canvas v0 now provides
`Color`, `Rect`, a software `Canvas`, deterministic drawing/checksum, and a
Win32 `Window` presenter; events, text/images, transforms, and other backends
remain future.

## Priority after the 2026-07-30 checkpoint

1. Extend the first lifecycle explain-chain with path reasons and scope cleanup.
2. Consider `select` only after the ordinary and timed blocking boundaries are
   stable.
3. Add long-lived File/Port/device handles only together with concrete APIs.
4. Design the first ESP32/FreeRTOS platform slice and hardware interrupt binding.
5. Add explicit `Ring`/bounded `Pool` semantics for `drop oldest`.
6. Grow Canvas with events, text/images, and additional presenters.
7. Resolve remaining lexer-only reservations and package/module ergonomics.

In parallel, the tooling track now has its first structured analysis facts and a
source-order ownership/resource/Task/Memory lifecycle chain in the TUI. Next are
dedicated Task/Channel/Memory views, path-sensitive explanations, and source
mapping plus debug probes for the first breakpoint/step/locals workflow. The
same engine must serve CLI, TUI, future LSP, and CI.

Every new form must update parser, semantic, codegen/runtime, formatter,
highlighting, quick reference, syntax status, and positive/negative/e2e tests.
Forms that create lifecycle, blocking, or ownership facts must also update the
structured analysis consumers.
