# Ветвления и циклы

## `if / else`

```skadi
if temperature > 30 {
    output("hot")
} else if temperature < 0 {
    output("cold")
} else {
    output("normal")
}
```

Условие должно иметь тип `Bool`.

## `when / is`

```skadi
when status {
    is 0 {
        output("ok")
    }
    is 1 {
        output("retry")
    }
    else {
        output("unknown")
    }
}
```

`when` вычисляет marker один раз и последовательно сопоставляет его с `is`.

## Циклы

Канонический обход коллекции:

```skadi
iterate values as value {
    output(value)
}
```

Рабочая альтернативная форма:

```skadi
for value in values {
    output(value)
}
```

Остальные циклы:

```skadi
while cursor < len(values) {
    cursor++
}

loop {
    if finished {
        break
    }
}
```

Внутри цикла доступны `break` и `continue`. `pass` задаёт намеренно пустой
statement или branch.

## Что не использовать

```skadi
for (i = 0; i < 10; i++) {
    output(i)
}
```

C-style форма сохраняется для чтения и форматирования старого кода, но semantic
отклоняет её через `SC-SEM-040`. Новый код должен использовать `iterate`,
`for ... in` или `while`.
