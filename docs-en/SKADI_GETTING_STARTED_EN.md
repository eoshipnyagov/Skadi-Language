# Quick Start

Skadi is a compiled systems language with readable syntax, explicit recovery
and ownership boundaries, and predictable lowering to C.

## Language idea

Skadi aims to combine:

- approachable application code;
- visible cost of systems operations;
- controlled memory without a hidden garbage collector;
- message-passing-first concurrency;
- one project workflow from source to a native binary.

The language does not collect features merely because other languages have
them. Generics, decorators, pipelines, implicit async runtimes, and unrestricted
operator overloading are outside the current model.

## Install

Release installers and archives cover Windows, Linux, and macOS. See
[Installation](installation.en.md) for checksums, updates, and uninstall.

```powershell
skadi-cli --version
skadi-cli doctor
```

Skadi emits C, so `build` and `run` need a supported C compiler. `check` does not.

## First standalone file

Small examples and utilities do not need a manifest:

```powershell
skadi-cli quick-run hello.skd
```

`quick-run` uses a temporary native build and removes its artifacts after
execution. An installed user does not need Cargo. Put program arguments after
`--`.

## First project

From your working directory:

```powershell
skadi-cli new hello_skadi
cd hello_skadi
skadi-cli check
skadi-cli run
```

```skadi
new Text greeting = "Hello from Skadi"
output(greeting)

new Angle quarter_turn = 90deg
output("Quarter turn: ", rad_to_deg(quarter_turn), " degrees")
```

## Everyday workflow

```powershell
skadi-cli check
skadi-cli format
skadi-cli build
skadi-cli run
```

Use `skadi-cli tui` for the interactive keyboard-first workflow. Regular CLI
commands remain the automation and CI surface.

## Minimal language sample

```skadi
fn sum_positive(Int List values) returns Int {
    new Int total = 0

    iterate values as value {
        if value > 0 {
            total = total + value
        }
    }

    return total
}

new Int List samples = [3, -1, 8]
output(sum_positive(samples))
```

## Where to go next

1. [CLI/TUI reference](cli-reference.en.md)
2. [Complete quick language reference](language-quick-reference.en.md)
3. [Topic-based language reference](language-reference.en.md)
4. [Recommended practices](practices.en.md)
