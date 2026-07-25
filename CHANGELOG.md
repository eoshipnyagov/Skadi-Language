# Changelog

All notable user-facing changes to Skadi are recorded here.

The project follows Semantic Versioning for compiler and CLI releases. Language
surfaces explicitly marked experimental can still evolve between minor
versions.

## [1.2.0-rc.1] - 2026-07-25

### Added

- Executable Memory MVP with fixed-capacity regions, `place in`, recovery, and
  deterministic `clear`.
- Native Task runtime on Windows and POSIX platforms, including result-bearing
  tasks, cooperative stop, repeated task creation, and required lifecycle
  checks.
- Typed bounded Channels with FIFO ordering and backpressure.
- Nominal `Time`, `Duration`, `ByteSize`, and `Angle` values.
- Built-in `Vec2`, `Vec3`, and `Vec4` value types with bounded vector math.
- ASCII `Char` literals and escapes.
- Full-screen project-oriented TUI.
- Cross-platform release archives, checksum verification, and user-local
  installers.

### Changed

- Compound assignments are accepted and formatted to their explicit canonical
  form.
- Legacy typed returns are accepted with a warning and formatted with
  `returns`.
- Untyped scalar declarations now receive the correct generated C type.
- Composite declarations require an explicit type.
- Compiler and CLI package identity is unified at `1.2.0-rc.1`.

### Safety and diagnostics

- `on interrupt` and legacy C-style `for` are rejected before code generation
  instead of disappearing from generated C.
- Memory, Task, and Channel ownership boundaries have explicit semantic
  diagnostics and native regression coverage.
- ThreadSanitizer is a required concurrency gate on Linux.

### Compatibility

- The stable `v1.1` CLI, formatter, diagnostics, math core, and project workflow
  remain available.
- Memory, Task/Channel, Time/Duration, ByteSize, Angle, and Vector surfaces are
  released as experimental `v1.2` tracks.

[1.2.0-rc.1]: https://github.com/eoshipnyagov/Skadi-Language/releases/tag/v1.2.0-rc.1
