# Embedded Platform Status

Skadi is designed with embedded use in mind, but `v1.2.0-rc.1` is not yet an
ESP32/MCU toolchain.

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
