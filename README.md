# Skadi

![CI](https://github.com/eoshipnyagov/Skadi-Language/actions/workflows/ci.yml/badge.svg)
![Docs](https://github.com/eoshipnyagov/Skadi-Language/actions/workflows/docs-site.yml/badge.svg?branch=master)
![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)

Documentation: [GitHub Pages](https://eoshipnyagov.github.io/Skadi-Language/)

**Skadi is an experimental systems language and toolchain focused on calm readability, explicit behavior, and practical native workflows.**

The current implementation is a working prototype: lexer, parser, semantic analysis, formatter, CLI/TUI, documentation tooling, and a practical `Skadi -> C` backend.

The current stable base is the `v1.1` toolchain surface. Active development is now focused on the `v1.2` experimental systems layer: executable Memory and Task/Channel runtime MVPs.

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
- experimental Memory MVP work for `v1.2`,
- experimental native Task/Channel runtime for `v1.2`,
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
    Ok
    ZeroDivision
    InvalidInput
}
```

```skadi
new Int value = safe_div(10, divisor) on error {
    output("division failed")
    return 0
}
```

```skadi
fn sum_positive(List(Int) xs) returns Int {
    new Int total = 0

    iterate xs as x {
        if x > 0 {
            total += x
        }
    }

    return total
}
```

The exact syntax is still evolving. The important part is the direction: readable, explicit, low-noise systems code.

## Tiny Syntax Contrast

<details>
<summary>Skadi</summary>

```skadi
fn sum_positive(List(Int) xs) returns Int {
    new Int total = 0

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

    window.present()
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
Channel(SensorData) sensors = channel(8)

Task sensor_task = run sensor_loop(sensors)

loop {
    new data = sensors.receive()
    draw_status(canvas, data)
    screen.present()
}

wait sensor_task
```

The intended model:

```text
Task runs work.
Channel carries messages.
stop requests shutdown.
wait joins and returns result.
shared mutable memory is not the default.
```

The current C backend maps each task to a Win32 or pthread native thread and
implements blocking bounded channels. Advanced scheduling, channel close,
timeouts, and embedded/RTOS targets remain future work. See the
[Concurrency Guide](https://eoshipnyagov.github.io/Skadi-Language/en/user/concurrency/).

### 3. Canvas

**Status: design direction / future versions.**

Skadi is planned to treat visual output as a core systems capability.

A future `Canvas` is not intended to be a full UI framework or a game engine.
It is intended as a small deterministic drawing surface for displays, windows, framebuffers, debug views, tools, and operator panels.

Possible future syntax:

```skadi
fn draw_status(Canvas canvas, Float temperature, Bool alarm) {
    canvas.clear(Color.black)

    canvas.text(Vec2(4, 4), "Temperature")
    canvas.text(Vec2(4, 18), concat(text(temperature), " C"))

    new Int width = clamp(Int(temperature * 2), 0, 100)
    canvas.rect(Rect(4, 40, 100, 10), Color.gray)
    canvas.fill_rect(Rect(4, 40, width, 10), Color.green)

    if alarm {
        canvas.text(Vec2(4, 56), "ALARM", Color.red)
    }
}
```

The same drawing logic should eventually be able to target:

- a small OLED display,
- a desktop window,
- an offscreen framebuffer,
- a game debug overlay,
- an operator panel.

### 4. Time and Units

**Status: design direction / future versions.**

Skadi should avoid hiding meaning inside bare numbers.

Possible future syntax:

```skadi
delay(500ms)
sleep(10min)

Memory log_memory = memory(32kb)

canvas.rotate(30deg)
```

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

If `skadi-cli` is already on your `PATH`:

```bash
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
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
skadi-cli build
skadi-cli run
skadi-cli format
skadi-cli tui
skadi-cli target list
```

## Documentation

User-facing docs:

- [User docs](docs/SKADI_DOCS_USER_RU.md)
- [Getting Started](docs/SKADI_GETTING_STARTED_RU.md)
- [CLI Quick Start](docs/SKADI_CLI_QUICK_START_RU.md)
- [CLI Reference](docs/SKADI_CLI_REFERENCE_RU.md)
- [Language Reference](docs/SKADI_LANGUAGE_REFERENCE_RU.md)
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
- experimental Memory MVP work for `v1.2`,
- experimental native Task/Channel runtime for `v1.2`,
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
