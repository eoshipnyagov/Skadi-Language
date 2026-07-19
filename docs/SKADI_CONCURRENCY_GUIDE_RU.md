# Многопоточность в Skadi

Статус: experimental systems surface `v1.2`, доступный в текущем компиляторе и
C backend.

Skadi использует модель задач и обмена сообщениями. Пользователь работает с
`Task` и `Channel(T)`, а не с платформенными дескрипторами потоков. На Windows и
POSIX host каждая запущенная задача сейчас выполняется в отдельном native thread.

## Быстрый пример

```skadi
fn calculate(Int value) returns Int {
    return value * value
}

Task(Int) first_task = run calculate(3)
Task(Int) second_task = run calculate(4)
new Int first = wait first_task
new Int second = wait second_task
output(first + second)
```

Обе задачи запускаются до первого `wait`, поэтому могут выполняться параллельно.
`wait` дожидается завершения, забирает результат и освобождает runtime-ресурсы
задачи.

## `Task` и `Task(T)`

Задача без результата имеет тип `Task`:

```skadi
fn save_report(Text report) {
    write("report.txt", report)
}

Task save_task = run save_report("ready")
wait save_task
```

Задача с результатом имеет тип `Task(T)`:

```skadi
fn load_status() returns Text {
    return "ready"
}

Task(Text) status_task = run load_status()
new Text status = wait status_task
```

Тип результата функции и параметр `Task(T)` должны совпадать. Для `Task(T)`
результат нужно получить выражением `wait`; обычный `wait task` используется для
задачи без результата.

`danger fn` пока нельзя использовать как task entry: перенос `ErrorCode` через
task boundary ещё не имеет отдельного контракта.

## Несколько параллельных задач

В языке нет специальной конструкции `run 5`. Нужно создать пять независимых
handle. Важно сначала запустить всю группу и только потом ожидать её:

```skadi
fn square_and_send(Int value, Channel(Int) results) {
    results.send(value * value)
}

Channel(Int) results = channel(2)
Task first_task = run square_and_send(1, results)
Task second_task = run square_and_send(2, results)
Task third_task = run square_and_send(3, results)
Task fourth_task = run square_and_send(4, results)
Task fifth_task = run square_and_send(5, results)

new Int total = 0
new Int received = 0
while received < 5 {
    new Int value = results.receive()
    total = total + value
    received++
}

wait first_task
wait second_task
wait third_task
wait fourth_task
wait fifth_task
output(total)
```

Текущий desktop runtime не задаёт искусственный лимит количества задач, но каждая
задача занимает системный поток и stack. Практический предел определяется ОС,
памятью и C toolchain. Thread pool и work-stealing scheduler пока отсутствуют,
поэтому тысячи коротких задач не являются рекомендуемым сценарием.

Порядок старта, завершения и обычного `output` из разных tasks не определён.
Детерминированным должен быть протокол обмена и итог после всех обязательных
`wait`, а не последовательность планирования потоков.

Если написать `run`, сразу `wait`, а затем следующий `run`, работа будет
последовательной:

```skadi
Task(Int) first_task = run calculate(3)
new Int first = wait first_task
Task(Int) second_task = run calculate(4)
new Int second = wait second_task
```

## Каналы

`Channel(T)` передаёт value-safe сообщения между задачами:

```skadi
struct Reading {
    Int sensor_id
    Float value
}

Channel(Reading) readings = channel(8)
readings.send(reading)
new Reading next_reading = readings.receive()
```

`channel(N)` создаёт bounded FIFO с явно заданной ёмкостью:

- `N` должен быть больше нуля;
- `send` блокируется, когда буфер заполнен;
- `receive` блокируется, когда буфер пуст;
- сообщения из одного канала выдаются в FIFO-порядке;
- несколько producers и consumers могут безопасно использовать один канал.

При нескольких producers FIFO отражает фактический порядок успешных `send`, а не
порядок строк `run` в исходнике. Гарантия fairness между ожидающими потоками пока
не задаётся.

Положительная ёмкость проверяется runtime. Нарушение завершается диагностикой
`SC-RT-312`.

### Backpressure

Небольшой буфер намеренно замедляет producer, если consumer не успевает:

```skadi
Channel(Int) values = channel(1)
```

Это встроенный backpressure. В текущем surface нет `try_send`, `try_receive`,
timeout, `select` и `close`.

### Завершение consumer

Канал не закрывается автоматически. Consumer должен знать количество сообщений
или получать явное значение протокола завершения. Если consumer ждёт сообщение,
которое никто не отправит, программа заблокируется.

## Жизненный цикл handle

`Task` является линейным owning handle:

- результат каждого `run` нужно сохранить;
- каждый handle нужно `wait` ровно один раз на всех путях выполнения;
- перед `wait` разрешён не более чем один `stop`;
- handle нельзя копировать, переприсваивать, помещать в `List`/`struct` или
  возвращать из функции;
- выход из scope с активным handle является semantic error `SC-SEM-070`;
- fire-and-forget и detached tasks не поддерживаются.

Корректный шаблон:

```skadi
Task worker_task = run worker()
wait worker_task
```

Некорректный шаблон:

```skadi
run worker()
```

Компилятор отклоняет проигнорированный handle, потому что иначе невозможно
гарантировать join и cleanup.

## Кооперативная остановка

`stop` публикует запрос на остановку, а `stopping` читает его внутри task entry:

```skadi
fn worker() {
    while not stopping {
        do_work()
    }
    cleanup()
}

Task worker_task = run worker()
stop worker_task
wait worker_task
```

`stop` не является принудительным убийством потока. Задача должна сама снова
дойти до проверки `stopping` и завершиться. После `stop` всё равно обязателен
`wait`.

Текущие блокирующие `receive`, `send` и файловый I/O не прерываются запросом
`stop`. Поэтому нельзя строить остановку worker так, чтобы он бесконечно ждал
пустой канал без отдельного сообщения завершения.

`stopping` разрешён только внутри функции, которую текущая программа запускает
через `run`. Повторный `stop` одного handle является semantic error.

## Повторный и периодический запуск

Завершённый handle нельзя запустить повторно. Для нового запуска той же функции
создаётся новый handle:

```skadi
new Int index = 0
new Int total = 0
while index < 5 {
    Task(Int) iteration_task = run calculate(index)
    new Int result = wait iteration_task
    total = total + result
    index++
}
```

Полный lifecycle `run -> wait` внутри одной итерации поддерживается. Нельзя
создать handle вне цикла, а `wait` или `stop` выполнять в зависимости от
количества итераций: compiler не сможет доказать единственный cleanup.

Этот шаблон повторяет работу, но не задаёт временной период. Стабильного
`sleep`/`delay`, timer API и единиц времени в текущем runtime ещё нет. Busy-wait
не рекомендуется. Настоящий периодический scheduler относится к будущему systems
time contract.

## Данные на границе задачи

Аргументы `run`, результат `Task(T)` и payload `Channel(T)` проверяются
рекурсивно.

Разрешены:

- числа, `Bool`, `Char`;
- `Text` и `Path` без region-owned payload;
- value-like `struct`, если все поля также безопасны;
- `Channel(T)` как специальная shared capability в аргументе task.

Запрещены:

- `Memory`;
- `Task` и вложенный `Channel` как сообщение;
- mutable `List` до появления определённой move/deep-copy семантики;
- region-owned значения из `place in`;
- структуры, рекурсивно содержащие запрещённые capability или payload.

Shared mutable memory не является моделью по умолчанию. Канал владеет общей
очередью, но переносимое сообщение копируется как value-safe representation.

Owning declaration `Channel(T) name = channel(N)` нужно размещать вне loop и
`place in`. Borrowed Channel-параметр можно использовать внутри worker и циклов.
Owner канала должен жить дольше всех задач, которым канал передан; все такие
задачи нужно `wait` до выхода из scope owner.

## Ошибки и взаимные блокировки

Compiler предотвращает потерю handle и небезопасную передачу данных, но не может
доказать отсутствие protocol deadlock.

Проверяйте следующие ситуации:

- consumer ожидает больше сообщений, чем producers отправляют;
- producer заблокирован на полном канале, а caller вызывает `wait producer` до
  чтения канала;
- две задачи ждут сообщения друг от друга;
- после `stop` worker остаётся в блокирующем `receive`;
- channel owner выходит из scope раньше задачи-пользователя.

Для диагностики проекта используйте:

```text
skadi-cli check
skadi-cli build
skadi-cli run
```

Frontend ошибки имеют `SC-SEM-*`, ошибки runtime — `SC-RT-*`, а проблемы C
toolchain отдельно помечаются CLI как toolchain failures.

Если runtime не может создать native thread или synchronization object, текущий
MVP выдаёт coded runtime diagnostic и завершает процесс. Recoverable
`run ... on error` пока отсутствует.

## Платформенная реализация

Текущий C backend:

- Windows: `CreateThread`, Win32 synchronization primitives и thread-local task
  context;
- Linux и другие поддерживаемые POSIX host: `pthread_create`, `pthread_join`,
  mutex/condition variable и thread-local context;
- один `Task` отображается на один native thread;
- `Channel` использует mutex и condition variables;
- `stop` публикуется потокобезопасно.

Это implementation detail текущего backend, а не вечное обещание языка. На
одноядерной системе задачи конкурентны, но не исполняются физически параллельно.
На многоядерном host ОС может выполнять их одновременно. Skadi пока не даёт
гарантий affinity, priority, stack size или real-time scheduling.

## ESP32 и микроконтроллеры

ESP32, ESP-IDF, FreeRTOS и bare-metal targets пока не поддерживаются официальным
CLI, backend и CI. Сгенерированный POSIX C нельзя считать готовым ESP32 port даже
при наличии pthread compatibility layer в ESP-IDF.

Предпочтительный путь реализации:

1. первым target family сделать ESP-IDF, отдельно для Xtensa и RISC-V chips;
2. добавить target profile и toolchain discovery в `skadi-cli`;
3. сначала проверить минимальный backend через ESP-IDF pthread compatibility;
4. затем сделать прямой FreeRTOS backend для управления stack, priority, core
   affinity и static allocation;
5. заменить безусловные heap allocations task context и channel buffer на
   настраиваемую static/region-backed политику;
6. добавить emulator smoke и hardware-in-the-loop tests.

До такого port нужно зафиксировать несколько design decisions: размер stack,
priority, pinning на core, лимит задач, источник памяти Channel и поведение при
исчерпании ресурсов. На ESP32 с одним core модель даст concurrency без настоящего
parallel execution; dual-core chips смогут исполнять задачи параллельно, если это
разрешит scheduler.

Текущий runtime не является hard real-time runtime. Кооперативный `stop`,
динамическое создание native threads и heap-backed channels не дают bounded
latency и deterministic allocation, необходимых для серьёзного embedded профиля.

## Текущие ограничения

Пока отсутствуют:

- task groups и structured-concurrency sugar;
- thread pool, work stealing и async/await;
- detached tasks;
- hard kill;
- channel close, timeout, `select`, `try_send`, `try_receive`;
- отмена блокирующего I/O или channel operation;
- timer, `sleep`/`delay` и периодический scheduler;
- affinity, priority и stack-size configuration;
- RTOS, ESP32 и bare-metal backend;
- гарантии hard real-time.

## Куда смотреть дальше

- [Справочник языка](language-reference.md)
- [Showcase-программы](showcases.md)
- `benchmarks/bench_11_task_channel_pipeline.skd`
- `benchmarks/bench_12_systems_pipeline.skd`
- `examples/concurrency/01_five_workers.skd`
- `examples/concurrency/02_restart_task.skd`
