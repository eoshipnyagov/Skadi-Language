# Skadi MVP: Контракт Memory Model

Дата: 2026-06-04  
Статус: рабочий MVP-контракт для реализации.

## 1. Назначение

Этот документ фиксирует не полную желаемую memory model Skadi, а именно тот минимальный контракт, который можно брать в parser/semantic/runtime design ближайшей реализации.

Связанные документы:

- [Memory Draft](memory-model-draft.md)
- [Memory Examples and Negative Cases](memory-model-examples.md)

Если нужен self-contained набор примеров без скрытого контекста, сначала стоит читать именно examples-документ.

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
9. упрощённое safety rule для возврата значений из `place in`;
10. ограниченно разрешённое nested `place in` для отдельного scratch-region внутри result-region.

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
fn make_numbers() Int List {
    new Int List numbers = []
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
place in level_memory {
    new Text file_text = read(path)
    output(file_text)
} on error {
    level_memory.clear()
    return LoadStatus.OutOfMemory
}
```

MVP-контракт:

```text
Если внутри региона не хватает места, аллокация попадает в общий on error блока place in.
Автоматический rollback уже созданных объектов не обещается: если нужен чистый регион, это делается явно через Memory.clear().
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
Внутри активного `place in` того же Memory вызов `clear()` считается ошибкой: канонический стиль — очищать регион после блока или в trailing `on error`.

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
struct LoadedText {
    Text content
}

fn load_text(Memory assets_memory, Path path) LoadedText {
    place in assets_memory {
        new Text file_text = read(path)
        new LoadedText result = {content = file_text}
        return result
    }
}
```

Запрещено:

```scadi
struct LoadedText {
    Text content
}

fn load_text(Path path) LoadedText {
    Memory scratch_memory = memory(4mb)

    place in scratch_memory {
        new Text file_text = read(path)
        new LoadedText result = {content = file_text}
        return result
    }
}
```

## 11. Nested `place in`

Nested `place in` в MVP разрешён только для одного практического сценария:

```text
Внешний region хранит результат.
Внутренний region обслуживает временную работу.
```

Это нужно, когда внутри одной операции есть два разных lifetime-класса данных:

- долгоживущий полезный результат;
- короткоживущие промежуточные аллокации.

Разрешено:

```scadi
place in assets_memory {
    place in scratch_memory {
        new Text preview_text = read(path)
        output(preview_text)
    } on error {
        scratch_memory.clear()
        output("scratch overflow")
    }

    new Text result_text = read(path)
    return result_text
}
```

MVP-правила:

- nested `place in` разрешён только между разными `Memory`;
- `place in` в ту же самую `Memory` внутри уже активного `place in` запрещён;
- внутренний `on error` ловит только overflow внутреннего блока;
- выход из внутреннего блока возвращает active region к внешнему;
- никакого автоматического rollback не обещается.

Запрещено:

```scadi
place in assets_memory {
    place in assets_memory {
        new Text msg = "bad"
        output(msg)
    }
}
```

Это считается ошибкой не потому, что runtime не справится, а потому, что same-memory nesting не даёт новой выразительности и только делает код менее прямолинейным.

## 12. Что именно обещает MVP

MVP memory model обещает:

- предсказуемый scope lifetime;
- явное создание регионов памяти;
- fixed-capacity поведение по умолчанию;
- явную обработку out-of-memory;
- явный `clear`;
- базовую защиту от возврата значения из умершего региона.
- `Memory` как special capability handle, а не как обычный storable value.

## 13. Что MVP пока не обещает

MVP memory model не обещает:

- полноценный borrow checker;
- общий lifetime calculus;
- child memory;
- `allow grow` как полноценно реализованную runtime-фичу;
- `allow drop` как полноценно реализованную runtime-фичу;
- сложный escape analysis;
- прозрачную региональную семантику для всех возможных value/reference edge cases.

И дополнительно не разрешает:

- `return Memory`;
- `Memory` в `struct`;
- `Memory List`;
- обычное копирование/переприсваивание `Memory` как regular value.

Nested `place in` при этом не обещает:

- глубокую произвольную матрёшку placement-блоков как рекомендуемый стиль;
- special semantics для same-memory nesting;
- дополнительную magic-логику поверх обычного active-region switch.

## 14. Рекомендация для реализации

Если брать этот контракт в работу, ближайший practical order такой:

1. Сначала зафиксировать syntax surface `Memory`, `memory(size)`, `place in`, `clear`.
2. Затем определить, какие типы в MVP вообще считаются region-relevant.
3. Затем реализовать semantic rule на возврат из `place in`.
4. Только после этого обсуждать `allow grow`, `allow drop`, `memory.child`, `memory.static`.

## 15. Короткая формула MVP-контракта

```text
Scope owns local values.
Return transfers ownership.
Memory owns groups of dynamic data.
place in chooses the region.
clear destroys the region contents.
Returning from a dead region is forbidden.
Nested place in is only for a shorter-lived scratch region inside a longer-lived result region.
```
