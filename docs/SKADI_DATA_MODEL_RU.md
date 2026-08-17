# Struct, Text и List

## Struct

```skadi
struct Counter {
    Int value
    hide Int limit

    fn bump(Int step) returns Int {
        my.value = my.value + step
        return my.value
    }
}

new Counter counter = {value = 0, limit = 10}
new Int current = counter.bump(2)
```

`hide` запрещает внешний доступ к полю, но методы того же struct используют его
через `my`. `local struct` не экспортируется из файла.

## Label и tag

`label` является числовым nominal-типом с обязательными явными уникальными
дискриминантами. `tag` является символическим nominal-типом без доступного
пользователю числового контракта.

```skadi
label ExitCode {
    Success = 0
    InvalidInput = 64
}

tag Direction {
    North
    South
}

new Direction direction = Direction.North

when direction {
    is North {
        output("north")
    }
    is South {
        output("south")
    }
}
```

Обе формы работают как типы значений, параметров и результатов функций.
Варианты не смешиваются с `Int` и с другими nominal-типами. Специальный
`label ErrorCode` использует тот же механизм, но дополнительно требует первым
вариантом `Ok = 0` для `danger/on error` flow.

Field punning сокращает literal, когда имя переменной совпадает с полем:

```skadi
new Float x = 2.0
new Float y = 4.0
new Point point = {x, y}
```

## Text и Path

```skadi
new Text full = concat("Skadi", " language")
new Bool found = contains(full, "language")
new Int at = find(full, "language")
new Text part = slice(full, 0, 5)
```

`Path` имеет text representation и помогает выразить назначение значения.
Текущий Text runtime byte-oriented; это важно для `len`, `slice` и индексации
UTF-8 текста.

## List

```skadi
new Int List values = [1, 2, 3]
values.push(5)

new Int last = 0
last = values.pop() on error {
    output("list is empty")
}
```

Тип записывается как `ElementType List`, не `List(ElementType)`.

```skadi
iterate values as value {
    output(value)
}
```

Списки mutable и поэтому не являются value-safe payload для `Channel`.
Для межзадачной передачи используйте scalar/specialized values или подходящий
immutable struct.
