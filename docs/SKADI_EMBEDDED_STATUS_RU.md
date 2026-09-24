# Embedded: платформенный статус

Skadi `v1.2` содержит первый экспериментальный ESP32/ESP-IDF target. Это уже
рабочий compiler/CLI slice, но ещё не завершённый и аппаратно
release-протестированный MCU toolchain.

## Принцип направления

Цель — не выставлять FreeRTOS отдельной пользовательской фичей, а сделать
обычную модель Skadi применимой на embedded-платформах. FreeRTOS, ESP-IDF или
другой RTOS остаётся backend-механизмом target и не протекает в Skadi-код
собственными task/queue/tick API.

Один и тот же словарь языка использует `Task`, `Channel`, `Duration`, `Memory` и
`on interrupt`. Различия платформ выражаются target-профилем, capabilities и
точными diagnostics, а не вторым диалектом языка.

## Что реализовано

- экспериментальный target `esp32-idf` с обычным 32-битным `Int`;
- генерация ESP-IDF application entry `app_main`;
- `Duration`, `sleep` и монотонное время поверх FreeRTOS/ESP Timer;
- `Task` поверх FreeRTOS tasks и `Channel` поверх bounded FreeRTOS queues;
- cooperative `stop`, `wait`, timed wait и проверка закрытия Channel на каждом
  RTOS tick;
- `interrupts.periodic(Duration)` поверх hardware GPTimer;
- `interrupts.gpio(pin, InterruptEdge.*, GpioPull.*)` поверх GPIO ISR;
- настоящий ISR handler для `on interrupt` и ISR-safe `Channel.try_send`;
- настраиваемые chip, Task stack/priority/core и стратегия размещения через
  `[embedded]`;
- статическое размещение Task context/TCB/stack/completion и Channel/queue;
- staging воспроизводимого ESP-IDF-проекта с declared native board adapters;
- CLI-команды `prepare`, `build`, `flash` и `monitor`;
- вертикальные примеры `examples/embedded-esp32-blink` и
  `examples/embedded-esp32-gpio`.

Generated C использует FreeRTOS и ESP-IDF напрямую, но исходная Skadi-программа
не содержит RTOS-типов или handles.

## Быстрый запуск ESP32-примера

Нужны ESP-IDF 5.4+ и активированная среда, в которой доступны `idf.py` и
`IDF_PATH`.

```powershell
cd examples/embedded-esp32-blink
skadi-cli embedded prepare
skadi-cli embedded build
skadi-cli embedded flash --port COM7 --monitor
```

`prepare` не требует установленного SDK: команда проверяет Skadi-код и создаёт
обычный ESP-IDF-проект в `build/esp32-idf`. `build`, `flash` и `monitor` запускают
`idf.py`. Путь к альтернативному launcher можно задать через `SKADI_IDF_PY`.

То же построение доступно общей командой:

```powershell
skadi-cli build --target esp32-idf
```

`--cc` для этого target не используется: C compiler, linker и chip configuration
выбирает ESP-IDF. `skadi-cli run --target esp32-idf` намеренно запрещён, потому
что firmware нельзя исполнить как host process.

## Вертикальный сценарий

Пример `embedded-esp32-blink` проверяет целевой поток целиком:

1. hardware GPTimer входит через `on interrupt`;
2. handler выполняет только interrupt-safe `try_send` в bounded Channel;
3. обычная Task получает событие, переключает GPIO и пишет в serial output;
4. board-specific GPIO остаётся в небольшом declared C adapter;
5. CLI создаёт, собирает, прошивает и открывает monitor одного ESP-IDF-проекта.

GPIO-вариант использует ту же границу ISR:

```skadi
Channel(Int) events = channel(4)
Interrupt button = interrupts.gpio(4, InterruptEdge.Falling, GpioPull.Up)

on interrupt button {
    events.try_send(4)
}
```

`InterruptEdge` поддерживает `Rising`, `Falling`, `Change`, `Low`, `High`;
`GpioPull` поддерживает `None`, `Up`, `Down`. Обработка события, I/O и прочая
обычная работа должны оставаться вне ISR.

## Embedded-настройки проекта

```toml
[embedded]
chip = "esp32"
allocation = "static"
task_stack_bytes = 4096
task_priority = 2
task_core = "any"
```

`allocation = "dynamic"` сохраняет прежнее поведение. При `static` емкость
каждого `channel(N)` должна быть положительным integer literal: это позволяет
compiler заранее сформировать queue storage. `task_core` принимает `"any"`,
`"0"` или `"1"`; неподдерживаемая конкретной микросхемой affinity будет
отклонена ESP-IDF при сборке/запуске.

## Текущая матрица

| Платформа | Статус |
|---|---|
| Windows x64 | Release-tested desktop |
| Linux x64 | Release-tested desktop |
| macOS x64/arm64 | Release-tested desktop |
| ESP32 / ESP-IDF | Experimental compiler/runtime/CLI slice; SDK build in CI, hardware smoke pending |
| Bare metal MCU | Future research |

## Ограничения первого slice

- ESP-IDF SDK build входит в CI, но flash/monitor ещё должны быть проверены на
  реальной ESP32-плате;
- static policy покрывает пользовательские Task и Channel, но Interrupt context
  и некоторые сервисы ESP-IDF пока могут использовать heap;
- `Memory.static` не заменяет отдельный embedded allocator contract;
- доступны GPTimer и GPIO interrupts, но общего device API для ADC, PWM, UART,
  SPI и I2C ещё нет;
- `[native].sources` поддерживаются, а `libraries` и `library_paths` для ESP-IDF
  пока отклоняются: зависимости board adapter задаются ESP-IDF components;
- `args`, input/file/directory I/O, Canvas и Window отклоняются до C toolchain с
  `SC-CG-303`; serial `output` и declared native adapters доступны;
- нет hard real-time guarantees, emulator/HIL matrix и embedded Canvas backend.

`SKADI_CHANNEL_POLL_TICKS` остаётся generated-C integration knob. Пользовательские
stack/priority/core/allocation задаются через `Skadi.toml`, без FreeRTOS-имен в
исходном Skadi-коде.

## Критерий завершённой поддержки

Target станет release-tested только после build/flash/monitor smoke на реальном
устройстве, CI или hardware-in-the-loop проверки, зафиксированной платы и
pinout и HIL-проверки. Наличие
профиля и успешно сгенерированного ESP-IDF-проекта само по себе не означает
hard-real-time или production-ready поддержку.
