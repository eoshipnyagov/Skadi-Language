# Skadi Task Model (Draft RU)

Дата: 2026-06-04
Статус: draft / design reference
Назначение: зафиксировать опорную модель задач, параллельного выполнения и межзадачной коммуникации в Skadi до полноценной реализации runtime/backend.

Связанный рабочий документ:

- [Task MVP Contract](task-model-mvp.md)

> Примечание о статусе: этот draft сохраняет ранние варианты решений. Текущий
> `v1.2` MVP-контракт строже: ignored `run` является hard error, а owning handle
> должен быть `wait`-нут на всех путях.

## 1. Основная идея

Skadi не должен начинать с понятия "поток ОС" как основной единицы конкурентности.

Основная сущность языка:

```scadi
Task
```

`Task` — это единица независимой, параллельной или псевдопараллельной работы.

Она может быть реализована по-разному:

```text
desktop OS       -> thread / thread pool
ESP32 / RTOS     -> RTOS task
bare metal       -> cooperative scheduler
game engine      -> job system
single-core MCU  -> соседний квант времени
```

Программист пишет:

```scadi
run sensor_loop()
```

а backend/runtime решает, как именно это исполнять на платформе.

Главная идея:

> Skadi описывает намерение "выполняй эту работу независимо", а не навязывает программисту детали `pthread`, `FreeRTOS`, fiber, coroutine или thread pool.

## 2. Главная формула модели

Короткая версия:

```text
Task runs work.
stop requests shutdown.
stopping observes shutdown.
wait joins and returns result.
Channel carries messages.
Shared memory is not the default.
```

По-русски:

```text
Task выполняет работу.
stop просит завершиться.
stopping показывает запрос остановки.
wait дожидается и возвращает результат.
Channel переносит сообщения.
Общая память не является обычным способом общения.
```

## 3. Минимальная модель

Для первого рабочего среза достаточно следующего ядра:

```text
run
wait
stop
stopping
Channel(T)
send
receive
```

То есть:

```text
run       -> запустить задачу
wait      -> дождаться задачи
stop      -> попросить задачу завершиться
stopping  -> флаг остановки внутри задачи
Channel   -> канал сообщений
send      -> отправить сообщение
receive   -> получить сообщение
```

Этого уже достаточно для большинства реалистичных сценариев:

- фоновое чтение датчиков;
- загрузка ассетов;
- приём входящих событий;
- логирование;
- выделенные service-задачи;
- обмен результатами с главным циклом.

И при этом модель не превращается сразу в большой async/futures/actors/runtime-комбайн.

## 4. `run` — запуск задачи

Базовый синтаксис:

```scadi
new task = run worker()
```

Смысл:

```text
Запусти worker как отдельную Task.
Верни handle задачи.
```

Пример:

```scadi
fn worker() {
    do_work()
}

fn main() {
    new task = run worker()

    do_other_work()

    wait task
}
```

`run` хорош тем, что он короткий, читаемый и не тащит исторический baggage платформенных API.

## 5. Task handle

Когда задача запускается с присваиванием:

```scadi
new task = run sensor_loop()
```

`task` — это handle задачи.

Через него можно:

```scadi
stop task
wait task
```

Handle делает lifecycle задачи явным:

- видно, кто владеет задачей;
- видно, кто может её остановить;
- видно, кто должен дождаться завершения.

## 6. Fire-and-forget не должен быть молчаливой нормой

Возможна запись:

```scadi
run blink_status_led()
```

Но для детерминированных систем это рискованно:

- задачу нельзя дождаться;
- задачу нельзя явно остановить;
- трудно понять, кто отвечает за её lifecycle.

Поэтому хорошее правило:

```text
Если результат run игнорируется — компилятор должен как минимум выдавать warning.
```

Например:

```text
warning: task handle ignored
```

Для MWP warning достаточно. Строже это можно делать позже.

## 7. `wait` — ожидание завершения

Базовый синтаксис:

```scadi
wait task
```

Смысл:

```text
Дождаться завершения задачи.
```

Пример:

```scadi
fn main() {
    new task = run calculate_path()

    update_ui()

    wait task
}
```

`wait` читается проще и нейтральнее, чем `join`, `await`, `future.get` или `handle.result`.

## 8. Задача с результатом

Если функция-задача возвращает значение, `wait` не только ждёт завершения, но и забирает результат.

```scadi
fn load_texture(Path path) returns Texture {
    return decode_texture(path)
}

fn main() {
    new texture_task = run load_texture(Path("player.png"))

    do_other_work()

    new texture = wait texture_task
}
```

Возможный явный тип:

```scadi
Task(Texture) texture_task = run load_texture(Path("player.png"))
```

Это удобно, потому что `wait` объединяет две привычные операции:

```text
join task
get result
```

в одну понятную конструкцию.

## 9. `stop` — кооперативная остановка

Базовый синтаксис:

```scadi
stop task
```

Смысл:

```text
Попросить задачу завершиться.
```

Важно:

```text
stop не убивает задачу насильно.
```

Это не `kill`.

Задача должна сама увидеть запрос остановки и завершиться в безопасной точке.

Пример:

```scadi
fn sensor_loop() {
    while not stopping {
        read_sensor()
        delay(100ms)
    }

    sensor.shutdown()
}

fn main() {
    new task = run sensor_loop()

    delay(10s)

    stop task
    wait task
}
```

Это принципиально важно для безопасности: задачу нельзя рвать посреди записи, device I/O, критического участка или выделения памяти.

## 10. `stopping` — флаг остановки текущей задачи

Внутри выполняемой задачи доступен флаг:

```scadi
stopping
```

Он показывает, что кто-то вызвал:

```scadi
stop task
```

Примеры:

```scadi
fn worker() {
    loop {
        if stopping {
            cleanup()
            return
        }

        do_work()
    }
}
```

или:

```scadi
fn worker() {
    while not stopping {
        do_work()
    }

    cleanup()
}
```

`stopping` хорошо читается и даёт прямую модель кооперативного shutdown.

## 11. `kill` не входит в базовую модель

В MWP не должно быть:

```scadi
kill task
```

Почему:

- это разрушает детерминированность;
- ломает cleanup;
- делает поведение runtime слишком опасным и платформозависимым.

Если когда-нибудь жёсткое убийство и понадобится, это должна быть либо явная dangerous-операция, либо platform/runtime-specific extension, а не часть базовой модели языка.

## 12. `Channel(T)` — основной способ общения задач

Основной тип коммуникации:

```scadi
Channel(T)
```

Пример:

```scadi
Channel(SensorData) sensors = channel(8)
```

Смысл:

```text
Канал для передачи сообщений типа SensorData.
Буфер канала — 8 сообщений.
```

Почему именно `Channel`:

- не тащит ассоциации с UNIX pipe;
- достаточно ясен как термин;
- хорошо описывает путь передачи сообщений между задачами.

Главный смысл:

> задачи не должны по умолчанию лезть в общую изменяемую память, а должны обмениваться сообщениями.

## 13. Создание `Channel`

Базовый синтаксис:

```scadi
Channel(Event) events = channel(32)
```

Смысл:

```text
Создать буферизированный канал для Event на 32 сообщения.
```

Примеры:

```scadi
Channel(SensorData) sensors = channel(8)
Channel(InputEvent) input = channel(32)
Channel(LogEntry) logs = channel(128)
```

Важный плюс: размер буфера виден в коде и становится частью runtime-архитектуры.

## 14. `send` — отправка сообщения

Базовый стиль:

```scadi
channel.send(message)
```

Пример:

```scadi
fn sensor_loop(Channel(SensorData) out) {
    while not stopping {
        new data = SensorData {
            temperature = read_temperature(),
            humidity = read_humidity()
        }

        out.send(data)

        delay(1s)
    }
}
```

Задача не изменяет чужое состояние напрямую. Она отправляет сообщение.

## 15. `receive` — получение сообщения

Базовый стиль:

```scadi
new value = channel.receive()
```

Пример:

```scadi
fn main() {
    Channel(SensorData) sensors = channel(8)

    new sensor_task = run sensor_loop(sensors)

    loop {
        new data = sensors.receive()
        display_temperature(data.temperature)
    }
}
```

Это делает коммуникацию явной и последовательной.

## 16. Блокирующее поведение по умолчанию

Для MWP разумно считать:

```text
send блокирует
receive блокирует
```

То есть:

- если буфер заполнен, `send` ждёт;
- если буфер пуст, `receive` ждёт.

Это простое, понятное и достаточно универсальное поведение для первого среза.

Неблокирующие варианты можно добавить позже как расширение.

## 17. `allow drop` для каналов

Для некоторых каналов допустима политика потери сообщений при переполнении:

```scadi
Channel(LogEntry) logs = channel(128, allow drop)
```

Смысл:

```text
Если буфер канала переполнен, сообщения можно терять,
вместо того чтобы блокировать критичный код.
```

Это особенно подходит для:

- логов;
- telemetry;
- вторичных отладочных событий;
- неключевых UI/event-каналов.

Главное достоинство: потеря сообщений становится не скрытой ошибкой, а явной политикой.

## 18. Shared mutable memory не должна быть моделью по умолчанию

Базовый concurrency-story Skadi не должен строиться вокруг общей изменяемой памяти.

То есть MWP должен исходить из принципа:

```text
message passing first
shared mutable state later, if ever
```

Это даёт:

- меньше скрытых race conditions;
- меньше runtime-хаоса;
- лучшую совместимость с embedded и game loop mindset;
- более читаемую архитектуру.

## 19. Value-сообщения как базовое ограничение MWP

Для MWP разумно ввести ограничение:

```text
Через Channel в MWP передаются только простые value-сообщения.
```

То есть без shared mutable state, shared references и сложной aliasing-семантики.

Это не значит "только Int и Bool". Это значит:

- value-like structs;
- сообщения, которые можно передать как изолированный пакет данных;
- без скрытого совместного владения изменяемым состоянием.

Это резко упрощает и модель, и реализацию.

## 20. Task ownership

Полезное правило:

```text
Тот, кто получил Task handle, отвечает за wait или за stop/wait.
```

Пример:

```scadi
new task = run worker()

stop task
wait task
```

Если задача сама завершается:

```scadi
wait task
```

Так lifecycle задач становится явной частью кода.

## 21. Task lifetime и scope

Если handle вышел из scope, а задача всё ещё работает, это проблема.

Плохо:

```scadi
fn bad() {
    new task = run worker()
}
```

Здесь handle умер, а задача может продолжать жить.

Возможные уровни строгости:

### Вариант A — warning

```text
warning: task may outlive its handle
```

### Вариант B — compile error

```text
error: task handle dropped without wait or stop
```

Для MWP warning уже полезен. В более строгой модели логичнее двигаться к compile-time error.

Главная мысль:

```text
Task handle нельзя молча потерять.
```

## 22. Structured concurrency как следующий шаг, но не ядро MWP

В будущем можно прийти к модели вроде:

```scadi
task group {
    run worker_a()
    run worker_b()
}
```

где при выходе из блока все дочерние задачи должны завершиться.

Это очень хорошо сочетается с философией Skadi:

```text
Если scope владеет памятью,
scope может владеть и задачами.
```

Но для MWP это пока лишний слой. Сначала нужен простой и прозрачный lifecycle через `run / stop / wait`.

## 23. Event syntax может быть позже

Позже можно добавить более удобный event-style слой поверх каналов:

```scadi
on sensors.message {
    display_temperature(message.temperature)
}
```

или:

```scadi
on channel.receive {
    handle(message)
}
```

Но это не должно быть ядром MWP.

Ядро:

```scadi
send
receive
```

Сначала простая и явная модель, потом удобный синтаксический слой, если он не создаёт лишней магии.

## 24. Errors и task result

Если задача возвращает статус:

```scadi
fn worker() returns WorkStatus {
    ...
}
```

то:

```scadi
new task = run worker()
new status = wait task
```

Пример:

```scadi
state WorkStatus {
    Done
    Failed
}

fn worker() returns WorkStatus {
    new ok = do_work()

    if ok {
        return WorkStatus.Done
    }

    return WorkStatus.Failed
}

fn main() {
    new task = run worker()

    new status = wait task

    when status {
        is WorkStatus.Done {
            output("done")
        }

        is WorkStatus.Failed {
            output("failed")
        }
    }
}
```

Skadi не обязан ради этого сразу вводить отдельную сложную task-exception model. Для первого среза задача может просто возвращать status/result.

## 25. Примеры

### 25.1. Embedded sensor task

```scadi
state SensorStatus {
    Ok
    Error
}

struct SensorData {
    f32 temperature
    f32 humidity
    SensorStatus status
}

fn sensor_loop(Channel(SensorData) out) {
    while not stopping {
        new data = SensorData {
            temperature = read_temperature(),
            humidity = read_humidity(),
            status = SensorStatus.Ok
        }

        out.send(data)

        delay(1s)
    }

    sensor.shutdown()
}

fn main() {
    Channel(SensorData) sensors = channel(8)

    new sensor_task = run sensor_loop(sensors)

    loop {
        new data = sensors.receive()

        when data.status {
            is SensorStatus.Ok {
                display_temperature(data.temperature)
            }

            is SensorStatus.Error {
                blink_error_led()
            }
        }
    }

    stop sensor_task
    wait sensor_task
}
```

Почему это хорошо:

- датчик работает независимо;
- главный код получает сообщения;
- нет общей изменяемой памяти;
- остановка кооперативная;
- буфер канала виден в коде.

### 25.2. Logger task

```scadi
struct LogEntry {
    Time time
    String text
}

fn logger(Channel(LogEntry) logs) {
    while not stopping {
        new entry = logs.receive()
        write_log(entry)
    }

    flush_log()
}

fn main() {
    Channel(LogEntry) logs = channel(128, allow drop)

    new log_task = run logger(logs)

    logs.send(LogEntry {
        time = now(),
        text = "System started"
    })

    run_application(logs)

    stop log_task
    wait log_task
}
```

Здесь `allow drop` честно говорит: при перегрузке системы логи можно потерять.

### 25.3. Game asset loader

```scadi
state LoadStatus {
    Ok
    FileError
    OutOfMemory
}

struct LoadRequest {
    Path path
}

struct LoadResult {
    Path path
    Texture texture
    LoadStatus status
}

fn asset_loader(
    Channel(LoadRequest) requests,
    Channel(LoadResult) results,
    Memory assets
) {
    while not stopping {
        new request = requests.receive()

        place in assets {
            new texture = load_texture(request.path) on error {
                results.send(LoadResult {
                    path = request.path,
                    texture = Texture.empty(),
                    status = LoadStatus.FileError
                })

                continue
            }

            results.send(LoadResult {
                path = request.path,
                texture = texture,
                status = LoadStatus.Ok
            })
        } on error {
            assets.clear()
            results.send(LoadResult {
                path = request.path,
                texture = Texture.empty(),
                status = LoadStatus.OutOfMemory
            })

            continue
        }
    }
}
```

Этот пример особенно важен, потому что связывает две ключевые идеи Skadi:

```text
Memory управляет lifetime ассетов.
Channel управляет коммуникацией задач.
```

### 25.4. Game input task

```scadi
state InputKind {
    KeyDown
    KeyUp
    MouseMove
}

struct InputEvent {
    InputKind kind
    Int code
    f32 x
    f32 y
}

fn input_loop(Channel(InputEvent) input) {
    while not stopping {
        new event = read_input_event()
        input.send(event)
    }

    input.shutdown()
}

fn game_loop() {
    Channel(InputEvent) input = channel(64)

    new input_task = run input_loop(input)

    loop {
        update_game()

        while input.has_messages() {
            new event = input.receive()
            handle_input(event)
        }

        render()
    }

    stop input_task
    wait input_task
}
```

`has_messages()` здесь можно считать будущей convenience-функцией, а не обязательной частью MWP.

## 26. Compiler rules / semantic rules

Если эта модель принимается как опорная, со временем компилятор должен поддерживать хотя бы такие правила:

1. Игнорирование результата `run` должно давать warning.
2. Потеря task handle без `wait` или `stop/wait` не должна быть молчаливой.
3. `stopping` доступен только внутри task-контекста.
4. `wait` применим только к task handle.
5. `stop` применим только к task handle.
6. Базовый `Channel(T)` в MWP должен принимать только value-safe сообщения.
7. Shared mutable memory не должна считаться стандартной concurrency-моделью по умолчанию.

## 27. Что не входит в MWP

Не нужно сразу включать:

- `async / await`;
- futures;
- promises;
- green threads;
- work stealing;
- actor system;
- `select`;
- `broadcast`;
- `mutex`;
- `atomic`;
- lock-free structures;
- thread priorities;
- cancellation tokens;
- task groups;
- scheduler configuration.

Иначе Skadi слишком рано уйдёт в concurrency research hell вместо рабочего и понятного runtime-story.

## 28. Сильные стороны модели

У этой модели очень сильные качества:

- низкая синтаксическая энтропия;
- видимый lifecycle задач;
- message passing вместо shared-state chaos;
- хорошая совместимость с embedded;
- хорошая совместимость с game loop mindset;
- понятная поверхность для MWP;
- отсутствие сильной привязки к одной OS/runtime-концепции.

Главное достоинство:

> Skadi делает многозадачность не магической runtime-системой, а видимой архитектурной частью программы.

## 29. Риски и точки аккуратности

При этом есть несколько мест, где нужно быть осторожными.

### 29.1. Нельзя слишком рано обещать "простую многопоточность"

Как только появляется настоящая конкурентность, возникает вопрос:

- что можно передавать между задачами;
- кто владеет данными;
- можно ли делиться памятью;
- как связать это с memory model.

Поэтому MWP должен быть максимально консервативным и message-first.

### 29.2. `allow drop` у каналов требует очень чёткой семантики

Если `allow drop` у канала означает "runtime иногда теряет что-то непонятно когда", модель быстро станет неотлаживаемой.

Лучше трактовать это просто:

```text
При переполнении буфера новые или старые сообщения могут быть отброшены
по явно зафиксированной policy.
```

И policy тоже лучше позже сделать явной.

### 29.3. Блокирующие `send/receive` просты, но надо помнить про deadlock story

Для MWP blocking semantics — хороший выбор. Но в дальнейшем надо будет решить:

- нужны ли timed receive/send;
- нужны ли try-операции;
- нужен ли select-like multiplexing.

Это пока не повод тащить всё сразу в ядро.

### 29.4. `stopping` — хорошее, но "магическое" имя

Это допустимо, если:

- оно локально и прозрачно;
- доступно только внутри task-контекста;
- не плодит дополнительные implicit runtime-объекты в пользовательской модели.

### 29.5. Связь с memory model критична

Нельзя обсуждать task model полностью отдельно от memory model.

Ключевой вопрос:

```text
Можно ли передать через Channel значение,
чьи динамические данные живут в Memory, не переживающей принимающую задачу?
```

Ответ почти наверняка должен быть "нет, без дополнительных жёстких правил".

## 30. Рекомендуемый путь внедрения

Практический порядок выглядит так:

1. Зафиксировать словарь:

   - Task
   - run
   - wait
   - stop
   - stopping
   - Channel(T)
   - send
   - receive
2. Сделать MWP только message-passing-first.
3. Разрешить в MWP только value-safe сообщения.
4. Дать blocking `send/receive` как поведение по умолчанию.
5. Сделать warning на игнорирование task handle.
6. Только потом обсуждать:

   - `try_send` / `try_receive`
   - `has_messages`
   - task groups
   - event syntax
   - scheduler tuning

## 31. Итоговая оценка

Как основная модель многозадачности для Skadi это очень хорошее направление.

Почему я считаю её сильной:

- она выражает намерение, а не OS-механику;
- она удерживает язык от преждевременного ухода в сложную async/runtime-теорию;
- она естественно сочетается с memory model;
- она очень хорошо подходит под embedded и game-oriented сценарии;
- она делает concurrency видимой и архитектурной, а не скрытой и магической.

Главная рекомендация:

> оставить MWP жёстко консервативным: Task + Channel + cooperative stop + value messages, без shared mutable state как нормы.

В таком виде task model выглядит не просто хорошей, а очень органичной именно для Skadi.
