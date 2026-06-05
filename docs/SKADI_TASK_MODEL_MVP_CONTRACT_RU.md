# Skadi MVP: Контракт Task Model

Дата: 2026-06-04  
Статус: рабочий MVP-контракт для реализации.

## 1. Назначение

Этот документ фиксирует не полную желаемую concurrency/task model Skadi, а именно минимальный контракт, который можно брать в parser/semantic/runtime design ближайшей реализации.

Широкая design-версия описана в [Task Draft](task-model-draft.md).

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
11. warning при игнорировании task handle;
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
new task = run worker()
```

MVP-контракт:

```text
run запускает задачу и возвращает handle.
```

Если функция задачи возвращает значение, handle концептуально типизирован этим результатом:

```scadi
Task(Texture) texture_task = run load_texture(path)
```

Даже если exact syntax ещё будет уточняться, сама семантика результата уже должна считаться частью контракта.

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
new texture_task = run load_texture(Path("player.png"))
new texture = wait texture_task
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
new event = events.receive()
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

MVP должен как минимум выдавать warning.

Например:

```text
warning: task handle ignored
```

Почему именно warning, а не error:

- это мягче для первого среза;
- уже помогает не терять lifecycle из вида;
- не блокирует дальнейшую эволюцию в более строгую сторону.

## 16. Что обещает MVP

MVP task model обещает:

- простой и читаемый запуск задачи;
- кооперативную остановку;
- видимый lifecycle через handle;
- message passing через typed channels;
- blocking communication semantics по умолчанию;
- предупреждение на fire-and-forget без явной ответственности.

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
