# Embedded Platform Status

Skadi is designed with embedded use in mind, but `v1.2.0-rc.1` is not yet an
ESP32/MCU toolchain.

The goal is not to expose FreeRTOS as a separate showcase feature. FreeRTOS,
ESP-IDF, or another RTOS is a target backend detail. User code should keep using
Skadi's `Task`, `Channel`, `Duration`, `Memory`, and `on interrupt` vocabulary
instead of RTOS-specific task, queue, or tick APIs.

| Platform | Status |
|---|---|
| Windows x64 | Release-tested desktop |
| Linux x64 | Release-tested desktop |
| macOS x64/arm64 | Release-tested desktop |
| WSL | Suitable Linux development environment |
| ESP32 / FreeRTOS | Planned; runtime and flash flow missing |
| Bare metal MCU | Future research |

A real platform port needs a compiler/linker profile, startup/runtime adapters,
time/task/channel/memory/I/O backends, semantic restrictions, flashing, and CI
or hardware-in-the-loop validation.

The proposed first slice is ESP32/FreeRTOS with RTOS-backed Duration, Task and
Channel, fixed/static memory policy, restricted I/O, and a hardware smoke
project.

That slice must be one useful vertical scenario rather than a collection of RTOS
wrappers: a hardware timer or GPIO event enters through `on interrupt`, the
handler performs only interrupt-safe work and `try_send`, a normal Task handles
the event, memory remains bounded, and the official workflow builds, flashes,
and verifies observable output. The slice is accepted only if Skadi source stays
calm and readable without FreeRTOS types or handles.
