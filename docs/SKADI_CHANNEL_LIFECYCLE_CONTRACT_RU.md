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
- `try_send()` на полном или закрытом канале возвращает `false`.

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

## 5. Отложено

- timeout;
- `select`;
- автоматический sender counting;
- cancellation blocking I/O;
- fairness guarantees;
- detached channel ownership.
