# Контракт `on interrupt`

Статус: **Accepted design, host и experimental ESP-IDF periodic MVP реализованы**.

## 1. Typed source

Interrupt binding выражается в программе typed capability, а не скрытым
сопоставлением строки в manifest:

```skadi
Interrupt tick = interrupts.periodic(10ms)

on interrupt tick {
    ticks.try_send(1)
}
```

Platform module может предоставить аппаратный источник:

```skadi
Interrupt button = esp32.gpio_interrupt(4, Rising)
```

Manifest выбирает target/backend, но не переименовывает обработчики.

## 2. Execution context

Handler исполняется в ограниченном interrupt context. Semantic pass
транзитивно запрещает:

- allocation;
- blocking;
- `sleep`, `delay`, `wait`, `stop`;
- blocking `send` и `receive`;
- file/network/general I/O;
- acquisition и release ресурсов;
- Canvas presentation;
- вызов функций без доказанного interrupt-safe контракта.

Разрешены scalar/value-safe вычисления и небольшой interrupt-safe API.
Основной первый bridge в normal context — `Channel.try_send` в заранее
созданный bounded channel.

## 3. Lifetime

Interrupt capability имеет owner. Регистрация handler действует не дольше
owner scope и снимается до освобождения captured capabilities. Handler не может
захватывать обычные mutable references; первый slice разрешает только явно
проверенные program-lifetime или capability captures.

## 4. Backends

- Windows/POSIX host backend предоставляет simulated periodic interrupt для
  semantic/runtime/showcase validation;
- backend не выдаётся за hardware IRQ;
- ESP-IDF backend связывает `interrupts.periodic` с hardware GPTimer callback и
  использует ISR-safe `Channel.try_send`;
- GPIO ISR и другие hardware sources пока не реализованы.

## 5. Отложено

- priorities и affinity;
- nested interrupts;
- dynamic handler replacement;
- arbitrary device registry;
- platform-specific interrupt payloads.
