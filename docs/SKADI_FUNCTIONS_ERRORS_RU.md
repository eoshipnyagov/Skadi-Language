# Функции, видимость и обработка ошибок

## Функции

```skadi
fn add(Int left, Int right) returns Int {
    return left + right
}

fn announce(Text message) {
    output(message)
    return
}
```

Параметры типизированы. Канонический возвращаемый тип записывается через
`returns`. Legacy-форма `fn name(...) Type` принимается только для
совместимости и переписывается formatter.

`local fn` видна только в текущем файле:

```skadi
local fn normalize_name(Text raw) returns Text {
    return raw
}
```

## Error flow

Ошибки функции объявляются явно:

```skadi
label ErrorCode {
    Ok = 0
    InvalidInput = 1
}

danger fn parse_positive(Int value) returns Int {
    if value <= 0 {
        return error InvalidInput
    }
    return value
}
```

Первый вариант `ErrorCode` обязан называться `Ok`. `return error` допустим
только внутри `danger fn`.

Вызывающая сторона задаёт recovery block:

```skadi
new Int parsed = 0
parsed = parse_positive(source) on error {
    output("invalid input")
}
```

Для danger-функции без полезного результата:

```skadi
save_state() on error {
    output("save failed")
}
```

Нельзя добавлять `on error` к обычному вызову или builtin I/O. Индексация также
пока остаётся fail-soft, а не danger operation.

## Практический принцип

Используйте `danger fn`, когда caller действительно обязан выбрать recovery
path. Не превращайте `ErrorCode` в скрытый exception mechanism: control flow
должен оставаться видимым в месте вызова.
