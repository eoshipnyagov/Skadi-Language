# Skadi MVP: Контракт Task Model

Дата: 2026-07-12
Статус: experimental / partial runtime MVP.

## 1. Назначение

Этот документ фиксирует не полную желаемую concurrency/task model Skadi, а именно минимальный контракт, который можно брать в parser/semantic/runtime design ближайшей реализации.

Широкая design-версия описана в [Task Draft](task-model-draft.md).

Backend-архитектура зафиксирована отдельно в
[Task Runtime MVP Design](task-runtime-mvp-design.md).

## 2. Что считается частью MVP

В MVP task model входят только:

1. `Task` как handle запущенной задачи;
2. `run fn(args)` как запуск задачи;
3. `wait task` как ожидание завершения;
4. `wait task` как получение результата, если задача его возвращает;
5. `stop task` как кооперативный запрос остановки;
6. `stopping` как флаг внутри выполняемой задачи;
7. `Channel(T)` как базовый способ коммуникации;
8. `channel(N)` как создание буферизированного канала;
9. `send` и `receive` как базовые операции обмена сообщениями;
10. blocking semantics для `send/receive` по умолчанию;
11. hard error при игнорировании task handle;
12. message-passing-first как основной concurrency story.

## 3. Базовая единица конкурентности: `Task`

MVP-правило:

```text
Task — это handle независимой работы, а не обещание конкретного OS-thread API.
```

То есть Skadi оперирует не "потоком ОС", а задачей, которую runtime/backend уже мапит на нужную платформенную механику.

## 4. `run`

Базовый синтаксис:

```scadi
Task worker_task = run worker()
```

MVP-контракт:

```text
run запускает задачу и возвращает handle.
```

Если функция задачи возвращает значение, handle типизируется этим результатом:

```scadi
Task(Texture) texture_task = run load_texture(path)
```

Это canonical syntax первого frontend milestone.

## 5. `wait`

Базовый синтаксис:

```scadi
wait task
```

MVP-контракт:

```text
wait дожидается завершения задачи.
Если задача возвращает результат, wait возвращает этот результат.
```

Пример:

```scadi
Task(Texture) texture_task = run load_texture(Path("player.png"))
new Texture texture = wait texture_task
```

## 6. `stop`

Базовый синтаксис:

```scadi
stop task
```

MVP-контракт:

```text
stop не убивает задачу насильно.
stop только просит задачу завершиться.
```

Это кооперативная остановка, не `kill`.

## 7. `stopping`

Внутри выполняемой задачи доступен флаг:

```scadi
stopping
```

MVP-контракт:

```text
stopping показывает, что для текущей задачи был запрошен stop.
```

Пример:

```scadi
fn worker() {
    while not stopping {
        do_work()
    }

    cleanup()
}
```

`stopping` должен быть допустим только внутри task-контекста.

## 8. `kill` не входит в MVP

MVP не включает:

```scadi
kill task
```

MVP-контракт:

```text
Жёсткое убийство задачи не является частью базовой модели Skadi.
```

Причина простая: это слишком опасно для предсказуемого cleanup и слишком быстро тащит платформозависимый хаос в базовую модель языка.

## 9. `Channel(T)`

Базовый синтаксис:

```scadi
Channel(Event) events = channel(32)
```

MVP-контракт:

```text
Channel(T) — это буферизированный канал для передачи сообщений типа T.
Размер буфера задаётся явно.
```

## 10. `send` и `receive`

Базовые операции:

```scadi
events.send(message)
new Event event = events.receive()
```

MVP-контракт:

```text
send отправляет сообщение в канал.
receive получает сообщение из канала.
```

## 11. Blocking semantics по умолчанию

Для MVP фиксируется простое поведение:

```text
send блокирует, если буфер канала заполнен.
receive блокирует, если буфер канала пуст.
```

Это deliberately simple default.

Неблокирующие формы вроде `try_send` / `try_receive` не входят в обязательный MVP-контракт.

## 12. Message passing first

Ключевое ограничение MVP:

```text
Shared mutable memory не является concurrency-моделью по умолчанию.
```

Основной путь общения задач в MVP:

```text
Task -> Channel -> Task
```

То есть Skadi в первой версии concurrency story опирается на обмен сообщениями, а не на общую изменяемую память.

## 13. Ограничение на сообщения в MVP

Для MVP разумно принять жёсткое правило:

```text
Через Channel в MVP передаются только value-safe сообщения.
```

Это означает:

- простые scalar values;
- value-like structs;
- данные без скрытого shared mutable ownership;
- без сложной aliasing-семантики.

Связь с memory model здесь принципиальна: сообщения не должны неявно уносить за собой dangling-region или shared-state проблемы.

## 14. Lifecycle task handle

MVP-правило:

```text
Тот, кто получил Task handle, отвечает за wait или за stop/wait.
```

Если задача завершилась сама:

```scadi
wait task
```

Если задачу нужно завершить снаружи:

```scadi
stop task
wait task
```

## 15. Игнорирование handle

Если результат `run` игнорируется:

```scadi
run worker()
```

Текущий frontend выдаёт hard error.

Например:

```text
Semantic error: [SC-SEM-070] task handle ignored
```

```text
Каждый run должен передать handle явному владельцу.
Каждый handle должен завершиться через wait на всех путях управления.
```

Иначе backend был бы вынужден неявно создавать detached task или скрытый process
registry, что противоречит явному lifecycle Skadi.

## 15.1. Текущий implementation status

В репозитории уже реализован frontend slice:

- parser/AST принимают `Task`, `Task(T)`, `run`, `wait`, `stop`, `stopping`, `Channel(T)`, `channel(N)`, `send` и `receive`;
- semantic layer проверяет lifecycle task handle, task-context для `stopping`, value-safe channel messages и запрет `Task` как обычного value-type;
- codegen исполняет void и result-bearing `run/wait` через Win32/pthread runtime;
- `Task(T)` переносит result из typed task context в ожидающий scope;
- `stop/stopping` работают как синхронизированный кооперативный запрос через
  thread-local current-task context;
- bounded `Channel(T)` исполняет blocking FIFO `send/receive` через
  Win32/pthread synchronization primitives;
- mutable `List` не считается value-safe сообщением до появления move/deep-copy
  контракта.
- channel owner создаётся вне loop и `place in`, чтобы `break/continue` или
  recovery jump не могли обойти deterministic cleanup; borrowed Channel-параметр
  использовать внутри таких блоков можно.

Task/Channel runtime уже является исполняемым, но всё ещё experimental slice.
Дальнейшее укрепление включает stress/sanitizer/CI matrix и showcase coverage;
`close`, cancellation, timeout и `select` остаются будущими контрактами.

Path-sensitive semantic pass принимает `wait` во всех ветках, отвергает cleanup
только в части веток, ранний `return` с живым handle и lifecycle, зависящий от
выполнения loop iteration.

## 16. Что обещает MVP

MVP task model обещает:

- простой и читаемый запуск задачи;
- кооперативную остановку;
- видимый lifecycle через handle;
- message passing через typed channels;
- blocking communication semantics по умолчанию;
- запрет fire-and-forget без явной ответственности.

## 17. Что MVP пока не обещает

MVP task model не обещает:

- `async/await`;
- futures/promises;
- actor system;
- `select`;
- `broadcast`;
- mutex/atomic/lock-free primitives;
- scheduler tuning;
- task groups;
- shared mutable state как норму;
- общую сложную exception/cancellation model;
- богатую систему неблокирующих channel-операций.

## 18. Рекомендация для реализации

Практический порядок такой:

1. Сначала зафиксировать syntax/AST surface: `run`, `wait`, `stop`, `stopping`, `Channel(T)`, `send`, `receive`.
2. Затем определить semantic rules для task handle и task context.
3. Затем зафиксировать ограничение на value-safe channel messages.
4. Затем решить, как эта модель будет стыковаться с `Memory`.
5. Только после этого обсуждать `try_send`, `try_receive`, `select`, task groups и event sugar.

## 19. Короткая формула MVP-контракта

```text
run starts work.
wait joins work and returns result.
stop requests cooperative shutdown.
stopping observes shutdown inside the task.
Channels carry value-safe messages.
Shared mutable state is not the default model.
```
