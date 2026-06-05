# Skadi MVP: Контракт Memory Model

Дата: 2026-06-04  
Статус: рабочий MVP-контракт для реализации.

## 1. Назначение

Этот документ фиксирует не полную желаемую memory model Skadi, а именно тот минимальный контракт, который можно брать в parser/semantic/runtime design ближайшей реализации.

Широкая design-версия описана в [Memory Draft](memory-model-draft.md).

## 2. Что считается частью MVP

В MVP memory model входят только следующие сущности и правила:

1. scope lifetime по умолчанию;
2. `return` как передача владения наружу;
3. `Memory` как явный регион памяти;
4. `place in Memory { ... }` как явное размещение;
5. `memory(size)` как fixed-capacity регион;
6. `on error` при невозможности создать `Memory`;
7. `on error` при нехватке места внутри `place in`;
8. `Memory.clear()` как уничтожение всего содержимого региона;
9. упрощённое safety rule для возврата значений из `place in`.

## 3. Scope lifetime по умолчанию

Базовое правило:

```text
Создано внутри scope -> уничтожено при выходе из scope.
```

Пример:

```scadi
fn process() {
    new buffer = List(u8)
    new text = Text("hello")
}
```

После выхода из `process` локальные значения считаются уничтоженными.

## 4. `return` передаёт владение

Если значение возвращается из функции, оно не уничтожается вместе с локальным scope функции.

```scadi
fn make_numbers() returns List(Int) {
    new numbers = List(Int)
    return numbers
}
```

MVP-правило:

```text
return передаёт владение вызывающему коду.
```

## 5. `Memory` как явная область памяти

Базовый синтаксис MVP:

```scadi
Memory level_memory = memory(256mb)
Memory frame_memory = memory(8mb)
Memory sensor_memory = memory(4kb)
```

MVP-контракт:

- `Memory` создаётся явно;
- размер задаётся явно;
- по умолчанию память fixed-capacity;
- автоматического роста по умолчанию нет.

## 6. Ошибка создания `Memory`

Создание памяти может закончиться ошибкой.

Пример:

```scadi
Memory level_memory = memory(256mb) on error {
    return LoadStatus.OutOfMemory
}
```

MVP-контракт:

```text
Если Memory создать нельзя, это не скрытая катастрофа runtime, а явная ошибка языка/runtime API.
```

## 7. `place in` как явное размещение

Базовый синтаксис MVP:

```scadi
place in level_memory {
    new level = Level(...)
}
```

MVP-контракт:

```text
Все динамические данные, созданные внутри блока, размещаются в указанной Memory.
```

`place in` отвечает за место размещения, а не за передачу владения.

## 8. Ошибка нехватки места внутри `place in`

Базовый синтаксис:

```scadi
place in level_memory on error {
    return LoadStatus.OutOfMemory
} {
    new texture = load_texture(path)
    new mesh = load_mesh(path)
}
```

MVP-контракт:

```text
Если внутри региона не хватает места, аллокация попадает в общий on error блока place in.
```

## 9. `Memory.clear()`

MVP допускает явную очистку региона:

```scadi
frame_memory.clear()
```

MVP-контракт:

```text
clear уничтожает всё содержимое выбранной Memory как группы.
```

Это group-destroy primitive, а не пообъектное освобождение.

## 10. Главный safety rule для MVP

Самое важное правило MVP:

```text
Нельзя вернуть значение, если его dynamic payload размещён в Memory,
которая умрёт раньше возвращаемого значения.
```

Но вместо полной общей lifetime-theory в MVP принимается упрощённое проверяемое правило:

```text
Значение, созданное внутри place in memory,
можно вернуть только если эта memory передана в функцию извне.
```

Разрешено:

```scadi
fn load_texture(Memory assets, Path path) returns Texture {
    place in assets {
        new texture = Texture(...)
        return texture
    }
}
```

Запрещено:

```scadi
fn load_texture(Path path) returns Texture {
    Memory temp = memory(4mb)

    place in temp {
        new texture = Texture(...)
        return texture
    }
}
```

## 11. Что именно обещает MVP

MVP memory model обещает:

- предсказуемый scope lifetime;
- явное создание регионов памяти;
- fixed-capacity поведение по умолчанию;
- явную обработку out-of-memory;
- явный `clear`;
- базовую защиту от возврата значения из умершего региона.

## 12. Что MVP пока не обещает

MVP memory model не обещает:

- полноценный borrow checker;
- общий lifetime calculus;
- child memory;
- `allow grow` как полноценно реализованную runtime-фичу;
- `allow drop` как полноценно реализованную runtime-фичу;
- сложный escape analysis;
- прозрачную региональную семантику для всех возможных value/reference edge cases.

## 13. Рекомендация для реализации

Если брать этот контракт в работу, ближайший practical order такой:

1. Сначала зафиксировать syntax surface `Memory`, `memory(size)`, `place in`, `clear`.
2. Затем определить, какие типы в MVP вообще считаются region-relevant.
3. Затем реализовать semantic rule на возврат из `place in`.
4. Только после этого обсуждать `allow grow`, `allow drop`, `memory.child`, `memory.static`.

## 14. Короткая формула MVP-контракта

```text
Scope owns local values.
Return transfers ownership.
Memory owns groups of dynamic data.
place in chooses the region.
clear destroys the region contents.
Returning from a dead region is forbidden.
```
