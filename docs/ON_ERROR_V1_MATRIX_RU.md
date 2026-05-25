# Skadi v1: Матрица `on error`

Дата: 2026-05-25  
Статус: зафиксированный контракт для `v1`.

## Разрешено в v1

1. Вызов `danger fn`:
- `x = danger_call(...) on error { ... }`
- `danger_call(...) on error { ... }`
- Семантическое правило: функция должна быть объявлена как `danger`.

2. Операция `List.pop()`:
- `x = xs.pop() on error { ... }`
- Используется как recoverable-путь для пустого списка.

## Не разрешено в v1

`on error` не применяется к обычным выражениям и non-danger вызовам, включая builtin-операции:

- `read(...) on error { ... }`
- `write(...) on error { ... }`
- `input(...) on error { ... }`
- `output(...) on error { ... }`
- `fs.list(...) on error { ... }`
- `fs.is_dir(...) on error { ... }`
- `fs.join(...) on error { ... }`
- `args() on error { ... }`
- `len/contains/find/slice/concat ... on error { ... }`
- индексация `xs[i]` / `t[i]` (для `v1` действует fail-soft контракт без `on error`).

## Поведение диагностики

Для non-danger call с `on error` semantic analyzer возвращает ошибку:
- `on error requires danger fn call: '<name>' is not declared as danger.`
- Для builtin-ов:
  - `on error requires danger fn call: builtin '<name>' is not danger in v1.`

Отдельно: dotted-форма (`fs.list(...) on error`) в текущем `v1` парсере не поддерживается как `danger on error` шаблон и отклоняется на parse-этапе.

## План после v1

- Возможное расширение `on error` на часть builtins/операций обсуждается для `v2+`.
- Для `v1` оставляем контракт узким и предсказуемым.

