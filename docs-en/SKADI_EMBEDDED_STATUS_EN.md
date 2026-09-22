# Embedded Platform Status

Skadi `v1.2` includes its first experimental ESP32/ESP-IDF target. This is a
working compiler and CLI slice, but not yet a hardware release-tested MCU
toolchain.

## Direction

The goal is not to expose FreeRTOS as a separate user feature. FreeRTOS,
ESP-IDF, or another RTOS remains a target backend detail. Skadi source keeps
using `Task`, `Channel`, `Duration`, `Memory`, and `on interrupt` instead of
RTOS-specific task, queue, or tick APIs.

## Implemented slice

- experimental `esp32-idf` target with a 32-bit default `Int`;
- ESP-IDF `app_main` generation;
- Duration, sleep, and monotonic time over FreeRTOS and ESP Timer;
- Task and bounded Channel backends using FreeRTOS tasks and queues;
- cooperative stop, wait, timed wait, and Channel-close checks every RTOS tick;
- `interrupts.periodic(Duration)` over hardware GPTimer;
- a real ISR handler for `on interrupt` and ISR-safe `Channel.try_send`;
- reproducible ESP-IDF project staging with declared native board adapters;
- CLI prepare, build, flash, and monitor commands;
- the `examples/embedded-esp32-blink` vertical example.

Generated C uses FreeRTOS and ESP-IDF directly, but the Skadi source contains no
RTOS types or handles.

## Running the example

Install ESP-IDF 5.4+ and activate an environment that provides `idf.py` and
`IDF_PATH`.

```powershell
cd examples/embedded-esp32-blink
skadi-cli embedded prepare
skadi-cli embedded build
skadi-cli embedded flash --port COM7 --monitor
```

`prepare` does not require the SDK: it checks the Skadi source and stages a
normal ESP-IDF project under `build/esp32-idf`. The other commands invoke
`idf.py`. Set `SKADI_IDF_PY` to use a different launcher.

The generic build path is also available:

```powershell
skadi-cli build --target esp32-idf
```

`--cc` does not apply to this target because ESP-IDF selects the C compiler,
linker, and chip configuration. `run --target esp32-idf` is intentionally
rejected because firmware is not a host process.

## Vertical scenario

The example validates one useful path rather than presenting a collection of
RTOS wrappers:

1. a hardware GPTimer enters through `on interrupt`;
2. the handler performs only interrupt-safe `try_send` into a bounded Channel;
3. a normal Task receives the event, toggles GPIO, and writes serial output;
4. board-specific GPIO stays in a small declared C adapter;
5. the CLI stages, builds, flashes, and monitors one ESP-IDF project.

## Platform matrix

| Platform | Status |
|---|---|
| Windows x64 | Release-tested desktop |
| Linux x64 | Release-tested desktop |
| macOS x64/arm64 | Release-tested desktop |
| ESP32 / ESP-IDF | Experimental compiler/runtime/CLI slice; hardware smoke pending |
| Bare metal MCU | Future research |

## Current limitations

- hardware build, flash, and monitor are not in CI and still need a real-board
  smoke pass;
- Task, Channel, and runtime contexts currently use dynamic ESP-IDF allocation;
  a static task/channel storage policy is not implemented;
- `Memory.static` is not a complete embedded allocator contract;
- periodic GPTimer is available, but GPIO interrupt sources and a general device
  API are not;
- `[native].sources` works, while ESP-IDF `libraries` and `library_paths` are
  rejected in favor of component dependencies;
- args, input/file/directory I/O, Canvas, and Window are rejected before the C
  toolchain with `SC-CG-303`; serial `output` and native adapters are available;
- stack size, priority, core affinity, and chip selection are not configurable
  through `Skadi.toml` yet;
- there are no hard real-time guarantees, emulator/HIL matrix, or embedded
  Canvas backend.

`SKADI_TASK_STACK_WORDS`, `SKADI_TASK_PRIORITY`, and
`SKADI_CHANNEL_POLL_TICKS` are generated-C integration knobs, not stable public
Skadi APIs.

The target becomes release-tested only after a real-device build/flash/monitor
smoke, CI or hardware-in-the-loop coverage, a documented board and pinout, and
an explicit bounded/static runtime allocation policy.
