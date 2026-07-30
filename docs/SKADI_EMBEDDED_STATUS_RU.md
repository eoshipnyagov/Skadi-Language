# Embedded: платформенный статус

Skadi проектируется с учётом embedded, но `v1.2.0-rc.1` пока не является
готовым ESP32/MCU toolchain.

## Что работает сейчас

- генерация переносимого C для core language;
- desktop targets и выбор внешнего C compiler;
- explicit `ByteSize`, `Duration`, `Angle`, root-static и fixed-capacity
  `Memory`; growing/child runtime доступен на desktop, но ещё не является
  embedded allocator backend;
- архитектурные ограничения, полезные для будущего bounded runtime.

## Что означает «поддержать платформу»

Одного target triple недостаточно. Для каждой платформы нужны:

1. профиль compiler/linker и output format;
2. startup/runtime adapter;
3. time, task, channel, memory и I/O backend;
4. platform-safe semantic restrictions;
5. upload/flash workflow;
6. CI emulator или hardware-in-the-loop validation.

## Текущая матрица

| Платформа | Статус |
|---|---|
| Windows x64 | Release-tested desktop |
| Linux x64 | Release-tested desktop |
| macOS x64/arm64 | Release-tested desktop |
| WSL | Подходит как Linux development environment |
| ESP32 / FreeRTOS | Planned; runtime и flash flow отсутствуют |
| Bare metal MCU | Future research |

## Предлагаемый первый embedded slice

ESP32/FreeRTOS выглядит практичным первым target:

- отдельный `esp32` profile;
- `Duration`/`delay` через RTOS tick API;
- `Task/Channel` adapter поверх FreeRTOS tasks/queues;
- static/fixed Memory policy;
- ограниченный I/O слой;
- sample project и hardware smoke.

До появления этого слоя desktop `Task/Channel` нельзя считать доказательством
embedded compatibility.
