# Контракт жизненного цикла Channel

Статус: **Accepted design, implementation track**.

## 1. Состояния

Bounded `Channel(T)` имеет два наблюдаемых состояния:

```text
Open -> Closed
```

`close()` означает, что новых сообщений больше не будет. Закрытие не уничтожает
очередь и не отбрасывает уже отправленные значения.

## 2. Операции

- `send(value)` блокируется при полном открытом канале;
- `try_send(value)` никогда не блокируется и возвращает `Bool`;
- `receive()` блокируется при пустом открытом канале;
- `close()` запрещает последующие отправки и будит ожидающие стороны;
- после `close()` consumer дочитывает буфер;
- `receive()` на пустом закрытом канале завершается через `on error`;
- `send()` на закрытом канале завершается через `on error`;
- `try_send()` на полном или закрытом канале возвращает `false`;
- `stop task` пробуждает именно эту task, если она заблокирована в `send` или
  `receive`; операция завершается через `on error`, а `stopping` сообщает причину.

## 3. Ownership

Owning declaration `Channel(T) q = channel(N)` создаёт owner. Переданный в
функцию или task параметр является borrowed capability.

- вызвать `close()` может только owner;
- owner должен пережить все использующие канал tasks;
- перед выходом из owner scope все такие tasks должны быть `wait`;
- runtime destruction выполняется после завершения пользователей;
- повторный `close()` безопасен на runtime boundary и может быть обработан через
  `on error`;
- повторный `close()` без handler отклоняется semantic pass, а заведомо
  закрытая операция с handler получает warning.

После условного `close()` состояние owner равно `maybe closed`. Операции
`send`, `receive` и `close` требуют `on error`; `try_send` остаётся безопасной
проверкой и возвращает `false`.

## 4. Канонический pipeline

```skadi
Task consumer = run consume(readings)
Task first = run produce_first(readings)
Task second = run produce_second(readings)

wait first
wait second
readings.close()
wait consumer
```

`stop` управляет task, а `close` завершает поток данных. Одна операция не
подменяет другую.

## 5. Cancellation blocking operation

Cancellation является состоянием task/операции, а не новым состоянием Channel:

- запрос `stop` постоянен и не закрывает Channel;
- ожидающая task регистрирует текущий Channel перед засыпанием;
- `stop` будит condition variable без polling;
- только операция остановленной task получает внутренний результат `Cancelled`;
- другие producers/consumers продолжают работу;
- отменённый `send` не добавляет сообщение, а отменённый `receive` не извлекает
  сообщение;
- если операция уже атомарно изменила очередь до `stop`, она считается успешно
  завершённой и не откатывается;
- после `stop` по-прежнему обязателен `wait`.

`Closed`, `Cancelled` и `TimedOut` различаются внутри runtime. На языковой
поверхности исходы входят в `on error`: внутри task `stopping == true` означает
cancellation, а в handler `send_for`/`receive_for` признак `timed_out` означает
deadline. Оставшийся путь соответствует `Closed`. Операция без handler при
фактической отмене обычного blocking вызова завершается `SC-RT-315`; timed-формы
требуют handler семантически.

Timed operation использует один абсолютный deadline: ложные пробуждения его не
продлевают. Успех уже доступной операции имеет приоритет над нулевым timeout;
`stop` после регистрации ожидания имеет приоритет над timeout. Перед возвратом
`TimedOut` runtime повторно проверяет очередь и `closed` под Channel lock.

## 6. Отложено

- timed `Task.wait`;
- `select`;
- автоматический sender counting;
- cancellation файлового и платформенного I/O;
- fairness guarantees;
- detached channel ownership.
