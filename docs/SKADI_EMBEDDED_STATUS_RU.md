# Embedded: платформенный статус

Skadi проектируется с учётом embedded, но `v1.2.0-rc.1` пока не является
готовым ESP32/MCU toolchain.

## Принцип направления

Цель — не добавить поддержку FreeRTOS как самостоятельную витринную фичу, а
сделать обычную модель Skadi применимой на embedded-платформах. FreeRTOS,
ESP-IDF или другой RTOS является backend-механизмом выбранного target и не
должен протекать в пользовательский код собственными task/queue/tick API.

По возможности один и тот же код продолжает использовать `Task`, `Channel`,
`Duration`, `Memory` и `on interrupt`. Различия платформ выражаются target-
профилем, доступными capabilities и точными diagnostics, а не вторым диалектом
языка.

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

Это должен быть вертикальный прикладной сценарий, а не набор разрозненных RTOS-
обёрток. Целевой smoke:

1. аппаратный timer или GPIO event входит через `on interrupt`;
2. handler выполняет только interrupt-safe работу и `try_send` в bounded Channel;
3. обычная Task обрабатывает событие и обновляет простой GPIO либо пишет в
   ограниченный serial output;
4. память и capacity известны заранее, а недоступные операции отклоняются до
   прошивки;
5. официальный flow выполняет build, flash и наблюдаемую проверку результата.

Первый slice принят только если Skadi-программа остаётся спокойной и читаемой,
не содержит FreeRTOS-типов и не требует понимания внутренних queue/task handles.
RTOS API может присутствовать в generated C и platform adapter, но не становится
параллельной пользовательской поверхностью языка.

До появления этого слоя desktop `Task/Channel` нельзя считать доказательством
embedded compatibility.
