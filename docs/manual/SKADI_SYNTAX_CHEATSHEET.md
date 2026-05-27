# Skadi Syntax Cheatsheet

Date: 2026-05-27

| Что нужно сделать | Как пишется в Skadi |
|---|---|
| Инициализация переменной | `new i32 name = 0` |
| Инициализация списка | `new i32 List nums = [1, 2, 3]` |
| Переопределить значение | `name = 42` |
| Инкремент | `i++` |
| Декремент | `i--` |
| Условие | `if x > 0 { ... } else { ... }` |
| Цикл while | `while i < n { ... }` |
| Цикл loop | `loop { ... }` |
| Цикл iterate | `iterate arrayname as i { ... }` |
| Совместимый for | `for item in items { ... }` |
| Выбор по значению | `when code { is 1 { ... } else { ... } }` |
| Функция | `fn add(Int a, Int b) Int { return a + b }` |
| Опасная функция | `danger fn parse(Text s) Int { ... }` |
| Обработка ошибки | `x = parse(t) on error { x = 0 }` |
| Вывод | `output(value)` |
| Чтение файла | `new Text body = read("in.txt")` |
| Запись файла | `write("out.txt", body)` |
| Импорт файла | `import "./lib.skd"` |
| Выход из цикла | `break` |
| Пропуск итерации | `continue` |
| No-op | `pass` |

Notes:
- `++`/`--` разрешены только как отдельные инструкции.
- В v1 каноничный импорт: только `import "./relative_path.skd"`.
