# Стилевое руководство Skadi v1

Черновик стилевых правил для кода и showcase-программ Skadi.

Цель простая: писать код, который читается без напряжения и без лишнего
визуального шума.

## 1. Циклы

- Предпочтительно: `iterate <collection> as <item>`
- Допустимо для совместимости: `for <item> in <collection>`

Пример:

```skadi
iterate entries as entry {
    output(entry)
}
```

## 2. Имена типов

- Фиксированные числовые типы пишутся в нижнем регистре:

  - `i8`, `i16`, `i32`, `i64`
  - `u8`, `u16`, `u32`, `u64`
  - `f32`, `f64`
- Читаемые типы пишутся с большой буквы:

  - `Int`, `Float`, `Bool`, `Char`, `Text`, `Path`, `List`, `Vec2`, `Vec3`, `Vec4`
- `bool` и `char` допустимы для совместимости, но в showcase-стиле предпочтительны `Bool` и `Char`.

## 3. Разбор CLI-флагов

- Предпочтительно использовать `when`.
- Длинных цепочек `if arg == ...` лучше избегать в showcase-коде.

Пример:

```skadi
when arg {
    is "--fast" { mode = "fast" }
    is "--safe" { mode = "safe" }
    else { }
}
```

## 4. Стиль вывода

- Для чисел и `Bool`: `output("label:")` + `output(value)`
- Для текста: `output(concat("label: ", text))`

Пример:

```skadi
output("count:")
output(count)
output(concat("file: ", path))
```

## 5. Ошибки и danger-flow

- Для потенциально опасных операций использовать `on error`.
- Имена error-кодов писать явно и предметно.

Пример:

```skadi
value = stack.pop() on error {
    output("stack is empty")
}
```

## 6. Приоритет для showcase-кода

- Для демонстрационных программ придерживаться одного каноничного стиля,
  даже если альтернативный синтаксис поддерживается парсером.


