# Migrating from Skadi v1.1 to v1.2

`v1.2` extends the stable `v1.1` tooling base. Existing projects do not require
a broad rewrite: the command remains `skadi-cli`, and the project layout and
core commands stay compatible.

## Post-upgrade checks

```bash
skadi-cli --version
skadi-cli doctor
skadi-cli check
skadi-cli format --check
skadi-cli build
```

The CLI should report `1.2.0-rc.1`. The `version` in `Skadi.toml` belongs to
the project and does not need to match the toolchain version.

## New capabilities

- fixed/growing/child/root-static `Memory`, `place in`, and an explicit overflow boundary;
- call-scoped `view`/`direct` and explicit `move` for current linear resources;
- native `Task`, `Task(T)`, `run`, `wait`, `stop`, and `stopping`;
- bounded `Channel(T)` with blocking `send/receive`, `try_send`, close/drain,
  cancellation, and Duration-bounded `send_for/receive_for`;
- path-sensitive `wait task for Duration on error { ... }` with handle
  preservation on timeout;
- nominal `Time`, `Duration`, `ByteSize`, and `Angle`;
- `Vec2`, `Vec3`, `Vec4`, and a bounded vector math slice;
- updated CLI/TUI with structured lifecycle analysis, documentation, and release
  installers.

These language surfaces remain experimental in the `v1.2` line even though
they are included in the distributed RC toolchain.

## Math compatibility

`sin` and `cos` still accept numeric radians for `v1.1` compatibility, but new
canonical code uses `Angle`:

```skadi
new Angle heading = 30deg
new Float direction_x = cos(heading)
```

## Stricter concurrency checks

A `run` result cannot be ignored. The task handle must be owned and joined:

```skadi
Task worker_task = run worker()
wait worker_task
```

Result-bearing tasks must use the matching type:

```skadi
Task(Int) result_task = run calculate()
new Int result = wait result_task
```

## Installed workflow

User documentation now leads with the installed command:

```bash
skadi-cli check
skadi-cli build
skadi-cli run
```

`cargo run -p skadi-cli -- ...` remains the source-tree workflow for compiler
contributors, not the primary end-user workflow.

## Rollback

Installers replace only the managed binary. To temporarily return to an older
published toolchain, run the installer with that explicit version. Project
sources are not modified.

See
[`CHANGELOG.md`](https://github.com/eoshipnyagov/Skadi-Language/blob/master/CHANGELOG.md)
for the complete change list.
