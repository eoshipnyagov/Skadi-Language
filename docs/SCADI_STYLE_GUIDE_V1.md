# Scadi Style Guide v1 (Draft)

Цель: писать код, который читается без напряжения, с минимальным визуальным шумом.

## 1. Циклы
- Предпочтительно: `iterate <collection> as <item>`
- Допустимо (совместимость): `for <item> in <collection>`

Пример:
```scadi
iterate entries as entry {
    output(entry)
}
```

## 2. Имена типов
- Фиксированные числовые типы: lowercase
  - `i8/i16/i32/i64`, `u8/u16/u32/u64`, `f32/f64`
- Человекочитаемые типы: Capitalized
  - `Int`, `Float`, `Bool`, `Char`, `Text`, `Path`, `List`, `Vec2`, `Vec3`, `Vec4`
- Совместимость: `bool`/`char` допустимы, но в витринном стиле рекомендуются `Bool`/`Char`.

## 3. Разбор CLI-флагов
- Предпочтительно через `when`.
- Избегать длинных цепочек `if arg == ...` в showcase-коде.

Пример:
```scadi
when arg {
    is "--fast" { mode = "fast" }
    is "--safe" { mode = "safe" }
    else { }
}
```

## 4. Стиль вывода
- Для чисел и bool: `output("label:")` + `output(value)`
- Для текстов: `output(concat("label: ", text))`

Пример:
```scadi
output("count:")
output(count)
output(concat("file: ", path))
```

## 5. Ошибки и danger-flow
- Для потенциально опасных операций использовать `on error`.
- Имена error-кодов писать явно и предметно.

Пример:
```scadi
value = stack.pop() on error {
    output("stack is empty")
}
```

## 6. Showcase-priority
- Для демонстрационных утилит придерживаться одного каноничного стиля,
  даже если альтернативный синтаксис поддерживается парсером.

