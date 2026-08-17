# Skadi

![CI](https://github.com/eoshipnyagov/Skadi-Language/actions/workflows/ci.yml/badge.svg)
![Docs](https://github.com/eoshipnyagov/Skadi-Language/actions/workflows/docs-site.yml/badge.svg?branch=master)
![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)

Documentation: [GitHub Pages](https://eoshipnyagov.github.io/Skadi-Language/)

**Skadi is an experimental systems language and toolchain focused on calm readability, explicit behavior, and practical native workflows.**

The current implementation is a working prototype: lexer, parser, semantic analysis, formatter, CLI/TUI, documentation tooling, and a practical `Skadi -> C` backend.

The current distributed line is `v1.2.0-rc.1`. It builds on the stable `v1.1`
toolchain surface and includes experimental Memory and ownership,
Task/Channel/Interrupt, Time/Duration, ByteSize, Angle, vector, and Canvas MVPs.

The long-term design direction is broader:

```text
Memory        explicit lifetime and region-oriented memory
Task/Channel  simple message-based concurrency
Canvas        a core visual output abstraction
Time/Units    readable time, sizes, angles, and physical values
```

These pillars describe where Skadi is going. Not all of them are implemented today.

## What Exists Today

Current Skadi is already useful as a language and toolchain experiment.

The repository includes:

- lexer,
- parser,
- semantic analysis,
- C code generation,
- `skadi-cli` as the main user interface,
- full-screen `skadi-cli tui`,
- formatter,
- math/core support for `v1.1`,
- relative path imports, `local`/`hide`, and qualified `module.symbol` access,
- experimental fixed/growing/child/root-static Memory runtime for `v1.2`,
- explicit `view`/`direct` borrows and `move` ownership transfer for current
  linear resources,
- experimental native Task/Channel runtime and periodic host Interrupts for
  `v1.2`, including cancellation-aware Channel operations and Duration-bounded
  Channel/Task waits,
- structured analysis facts with stable IDs/source anchors for blocking/timed
  operations, incomplete `when`, and resource lifecycle chains displayed in a
  dedicated TUI workspace and exported by `analyze --json`,
- statement-level Skadi-to-C debug maps with multi-file source origins, plus an
  opt-in probe debugger with source breakpoints, continue/step, call stacks,
  scalar locals, thread-aware cooperative stops, and a TUI debug workspace,
- experimental Time/Duration, ByteSize, Angle, and Vec2/Vec3/Vec4 types,
- target-configurable `Int`, fixed-width integer literals, and checked bit builtins,
- experimental software Canvas and Win32 window presenter,
- deterministic release archives and user-local installers,
- showcase programs,
- regression tests,
- RU/EN documentation scaffolding,
- HTML docs site structure.

The current focus is to make Skadi real enough to test syntax, diagnostics, examples, workflows, and the general feel of the language.

## Why Skadi Exists

Modern systems languages are powerful, but their surface often inherits a lot of syntactic noise from older traditions.

Skadi explores a different tradeoff:

- semantic clarity over symbolic compression,
- explicit behavior over hidden runtime magic,
- readable systems code over clever syntax,
- practical tooling over language-theory spectacle,
- deterministic structure over "it probably works".

Skadi is not trying to become "natural language programming".
It is trying to reduce the distance between the idea and the code.

> Less syntactic entropy. More signal.

## Current Philosophy

Skadi starts from a simple belief:

> systems programming should stay explicit, but it should not feel cluttered.

Compared with Rust, Skadi is less about deep ownership machinery and more about a calmer source-language surface and direct tooling.
Compared with Go, Skadi keeps more expression in the language itself and does not try to hide control flow behind a minimal syntax.
Compared with Zig, Skadi is less centered on comptime and low-level metaprogramming, and more focused on readable everyday systems programs.
Compared with C++, Skadi trades ecosystem scale and maximum raw flexibility for a more opinionated and less noisy workflow.

Skadi is trying to keep the useful parts of systems-level programming while making the default experience easier to read, easier to run, and easier to maintain.

## Current Language Taste

Skadi prefers words when they carry meaning better than symbols.

```skadi
danger fn safe_div(Int a, Int b) returns Int {
    if b == 0 {
        return error ZeroDivision
    }

    return a / b
}
```

The point is not to make code verbose.
The point is to avoid meaningless compression when a short word carries the meaning better.

Examples of the current intended style:

```skadi
label ErrorCode {
    Ok = 0
    ZeroDivision = 1
    InvalidInput = 2
}
```

```skadi
new Int divisor = 2
new Int value = 0
value = safe_div(10, divisor) on error {
    output("division failed")
    value = 0
}
```

```skadi
fn sum_positive(Int List xs) returns Int {
    new Int total = 0

    iterate xs as x {
        if x > 0 {
            total = total + x
        }
    }

    return total
}
```

Experimental systems syntax is still evolving, but examples in this README use
the current canonical compiler surface.

## Tiny Syntax Contrast

<details>
<summary>Skadi</summary>

```skadi
fn sum_positive(Int List xs) returns Int {
    new Int total = 0

    iterate xs as x {
        if x > 0 {
            total = total + x
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

## Design Direction

The following sections describe the intended direction of Skadi.
They are not a claim that every feature below is implemented today.

### 1. Memory

**Status: experimental `v1.2` frontend and native runtime MVP.**

Skadi makes memory an explicit architectural resource in the current experimental track.

The intended model is not "manual `malloc/free` everywhere" and not "hide everything behind a garbage collector".

The long-term goal:

```text
temporary data should die predictably
long-lived data should have a visible owner
groups of data should be clearable together
```

Current MVP syntax:

```skadi
Memory frame_memory = memory(16mb)

loop {
    frame_memory.clear()

    place in frame_memory {
        update_world(world)
        draw_world(canvas, world)
    } on error {
        output("frame memory overflow")
        continue
    }

    window.present(direct canvas)
}
```

Why this matters:

- games often have frame memory,
- embedded systems often need fixed buffers,
- tools often need temporary work memory,
- deterministic systems should not hide allocation behavior.

### 2. Task / Channel

**Status: experimental `v1.2` runtime MVP.**

Skadi uses tasks for independent work and bounded channels for message passing.

Current syntax:

```skadi
fn collect(Channel(Int) readings) {
    while not stopping {
        readings.send(1) on error {
            return
        }
    }
}

Channel(Int) readings = channel(1)
Task sensor_task = run collect(readings)
new Int sample = readings.receive()
stop sensor_task
wait sensor_task
output("sample: ", sample)
```

The intended model:

```text
Task runs work.
Channel carries messages.
stop requests shutdown.
wait joins and returns result.
shared mutable memory is not the default.
```

The current C backend maps each task to a Win32 or pthread native thread. It
supports multiple tasks, task restart after `wait`, bounded blocking channels,
non-blocking `try_send`, explicit channel close, and typed periodic host
Interrupts. `stop` also wakes a task blocked in Channel `send/receive` without
closing or draining the channel. `send_for`/`receive_for` bound waits with a
`Duration`; their `on error` handlers distinguish cancellation with `stopping`
and deadlines with `timed_out`. Timed Task wait consumes the handle on success
and preserves it in the timeout handler. Cancellation of file I/O, hardware IRQ
binding, advanced scheduling, and embedded/RTOS targets remain future work. See the
[Concurrency Guide](https://eoshipnyagov.github.io/Skadi-Language/en/user/concurrency/).

### 3. Canvas

**Status: experimental `v1.2` software Canvas and Win32 presenter MVP.**

Skadi treats visual output as a core systems capability in the current
experimental track.

`Canvas` is not intended to be a full UI framework or a game engine. The
current MVP is a small deterministic software drawing surface with colors,
rectangles, pixels, lines, circles, alpha blending, and a frame checksum.

Current syntax:

```skadi
Canvas frame = canvas(320, 200)
Window window = windows.open("Skadi Canvas", 320, 200)

new Color background = color_hex("#1d1f21", 255)
new Rect panel = rect(24.0, 24.0, 272.0, 152.0)
new Vec2 center = {x = 160.0, y = 100.0}

frame.clear(background)
frame.fill_rect(panel, Color.terminal_blue)
frame.circle(center, 48.0, Color.terminal_bright_yellow)

while window.is_open() {
    window.present(direct frame)
    sleep(16ms)
}
```

The software Canvas and headless checksum path are portable. The interactive
window presenter currently targets Win32. Future presenters are intended for:

- a small OLED display,
- additional desktop systems,
- direct framebuffers,
- a game debug overlay,
- an operator panel.

### 4. Time and Units

**Status: experimental `Time/Duration`, `ByteSize`, and `Angle` runtime MVPs; broader units remain future work.**

The implemented time slice avoids hiding meaning inside bare numbers.

Current and future unit syntax:

```skadi
delay(500ms)
new Duration hardware_tick = 1ns
sleep(10min)

new ByteSize log_capacity = 32kb
new ByteSize archive_capacity = 1tb
Memory log_memory = memory(log_capacity)

new Angle heading = 30deg
new Float direction_x = cos(heading)
```

`Time`, `Duration`, `ns`, `us`, `ms`, `s`, `min`, `h`, `now`, `elapsed`, `sleep`, and `delay` are
implemented. `ByteSize` is implemented as a nominal byte-count type with
`b/kb/mb/gb/tb` literals and `memory(ByteSize)` integration. `Angle` adds
`deg/rad` literals, nominal arithmetic, complete basic trigonometry, and explicit
degree/radian conversion. Broader
physical units remain design direction.

The goal is to reduce mistakes like:

```skadi
delay(1000) // 1000 what?
```

Time, memory sizes, angles, rates, and eventually selected physical units should be readable at the call site.

## Where Skadi Should Be Strong

Skadi is meant for software shaped like this:

```text
devices / input
      |
tasks and channels
      |
state
      |
canvas / display / output
      |
predictable lifecycle
```

Strong target areas:

- embedded application logic,
- firmware with visible state,
- operator panels,
- small visual tools,
- simulations,
- game systems,
- asset and debug tools,
- small-to-medium native applications,
- educational systems programming examples.

The short version:

> Skadi is for systems with form.

## Where Skadi Is Not Trying to Win

Skadi is intentionally not focused on:

- enterprise CRUD applications,
- large web backends,
- throwaway scripting,
- data science notebooks,
- highly dynamic object graphs,
- massive distributed systems,
- reflection-heavy runtime applications,
- lock-free shared-memory techniques as the default style.

This is not a weakness to hide.
It is part of the design boundary.

A focused language has to know what it is not.

## Toolchain

The main user entrypoint is:

```bash
skadi-cli
```

Skadi currently uses a practical `Skadi -> C` backend for portability.

The goal is not to build a perfect compiler backend first.
The goal is to make the language real enough to test syntax, semantics, diagnostics, examples, and workflows.

## Quick Start

Install the `v1.2.0-rc.1` release candidate first.

Windows PowerShell:

```powershell
$installer = Join-Path $env:TEMP "skadi-install.ps1"
Invoke-WebRequest `
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/install.ps1 `
  -OutFile $installer
& $installer -Version 1.2.0-rc.1
```

Linux or macOS:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/eoshipnyagov/Skadi-Language/v1.2.0-rc.1/install/install.sh \
  | sh -s -- --version 1.2.0-rc.1
```

The installer verifies the release SHA-256 and installs `skadi-cli` without
requiring Rust. A host C compiler is still required for `build` and `run`;
`skadi-cli doctor` reports the available toolchains.

Then, from a working directory:

```bash
skadi-cli quick-run hello.skd
```

`quick-run` builds and executes one `.skd` file without `Skadi.toml`, Cargo, or
persistent build artifacts. A host C compiler is still required. Use `--` to
forward program arguments: `skadi-cli quick-run hello.skd -- first second`.

For a regular project:

```bash
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
skadi-cli debug --break src/main.skd:1
```

For interactive work:

```bash
skadi-cli tui
```

If you want to run directly from the source tree, use Cargo as a fallback:

```bash
cargo run -p skadi-cli -- check
```

## CLI Commands

```bash
skadi-cli doctor
skadi-cli new <name>
skadi-cli init
skadi-cli check
skadi-cli analyze --json
skadi-cli build
skadi-cli run
skadi-cli debug [-b file.skd:line]
skadi-cli quick-run <file.skd> [-- <args>]
skadi-cli format
skadi-cli tui
skadi-cli target list
```

## Documentation

User-facing docs:

- [User docs](docs/SKADI_DOCS_USER_RU.md)
- [Installation](docs/SKADI_INSTALLATION_RU.md)
- [Migration from v1.1 to v1.2](docs/SKADI_V1_2_MIGRATION_RU.md)
- [Getting Started](docs/SKADI_GETTING_STARTED_RU.md)
- [CLI Quick Start](docs/SKADI_CLI_QUICK_START_RU.md)
- [CLI Reference](docs/SKADI_CLI_REFERENCE_RU.md)
- [Language Reference](docs/SKADI_LANGUAGE_REFERENCE_RU.md)
- [Short language examples](docs/SKADI_LANGUAGE_EXAMPLES_RU.md)
- [Time and Duration](docs/SKADI_TIME_DURATION_RU.md)
- [Byte Sizes](docs/SKADI_BYTE_SIZE_RU.md)
- [Angles](docs/SKADI_ANGLE_RU.md)
- [Integer model and bit operations](docs/SKADI_BITS_RU.md)
- [Ownership and borrowing](docs/SKADI_OWNERSHIP_RU.md)
- [Concurrency](docs/SKADI_CONCURRENCY_GUIDE_RU.md)
- [Canvas and Visual Core](docs/SKADI_VISUAL_CORE_MVP_CONTRACT_RU.md)
- [Showcase programs](docs/SHOWCASE_PROGRAMS.md)

Internal docs:

- [Internal docs](docs/SKADI_DOCS_INTERNAL_RU.md)
- [Project technical overview](docs/SKADI_PROJECT_OVERVIEW_RU.md)
- [Syntax status](docs/SKADI_SYNTAX_STATUS.md)
- [Diagnostics style](docs/DIAGNOSTICS_STYLE.md)
- [Implementation plan](docs/SKADI_IMPLEMENTATION_PLAN_RU.md)

Local HTML docs:

```text
scripts\open_docs.bat
scripts\open_docs.ps1
```

## Repository Layout

```text
src/              compiler core: lexer, parser, semantic analysis, codegen
tools/skadi-cli/  CLI and TUI frontend
docs/             user docs, internal docs, contracts, and plans
docs-en/          English docs layer for the HTML site
examples/         sample source files
benchmarks/       showcase and regression programs
tests/            unit, integration, and smoke tests
```

## Current Status

The repository already includes:

- lexer,
- parser,
- semantic analysis,
- C code generation,
- `skadi-cli` as the main user interface,
- full-screen `skadi-cli tui`,
- formatter,
- math/core support for `v1.1`,
- relative path imports, `local`/`hide`, and qualified `module.symbol` access,
- experimental fixed/growing/child/root-static Memory regions and explicit
  resource ownership for `v1.2`,
- experimental native Task/Channel runtime and periodic host Interrupts for
  `v1.2`, including blocking Channel cancellation and timed Channel/Task waits,
- structured compiler analysis facts surfaced by `analyze --json` and the TUI
  Lifecycle workspace,
- experimental nominal Time/Duration runtime for `v1.2`,
- experimental nominal ByteSize and dynamic Memory capacity for `v1.2`,
- experimental nominal Angle and `deg/rad` math integration for `v1.2`,
- experimental Vec2/Vec3/Vec4 values and vector math for `v1.2`,
- experimental software Canvas and Win32 window presenter,
- release archives and installers for Windows, Linux, and macOS,
- showcase programs,
- regression tests,
- RU/EN documentation scaffolding,
- HTML docs site structure.

Skadi is still experimental.

The implemented language, the design documents, and the long-term vision are not the same thing yet.
The README intentionally separates:

```text
current implementation
design direction
future language goals
```

## Design Principles

Skadi should stay small enough to understand.

Core features should earn their place by serving deterministic systems with visible state and predictable behavior.

Where a mature technical standard exists, Skadi follows it and makes deliberate
differences explicit. Where no useful UX is standardized, the language favors
practical completeness and readable behavior without hiding cost or ownership.

A feature belongs close to the core only if it helps express one of these things clearly:

```text
where data lives
who performs work
how messages move
what gets drawn
how time passes
which device is involved
what can fail
what is dangerous
```

Everything else should probably be a library.

## License

This project is licensed under the [MIT License](LICENSE).
