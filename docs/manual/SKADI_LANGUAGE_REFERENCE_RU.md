# Skadi: Документация Языка (текущее состояние v1-snapshot)

Дата актуальности: 2026-05-26
Статус: практическая документация по **реально реализованному** поведению.

## 1. Что уже работает

Реализован рабочий pipeline:
`Skadi source -> lexer -> parser -> semantic -> C codegen -> native compile (через skadi-cli)`.

Поддержаны базовые конструкции:
- `new`-объявления,
- функции `fn` и `danger fn`,
- `if/else`, `while`, `loop`, `when/is/else`,
- циклы `for item in list` и `iterate list as item`,
- `i++` / `i--` как отдельные инструкции,
- `break`, `continue`, `pass`,
- `label`,
- ограниченная поддержка `struct` (по текущему codegen-статусу).

## 2. Синтаксические основы

- Одна инструкция на строку (без `;`).
- Блоки: `{ ... }`.
- Комментарии: `//` и `/* ... */`.
- Вызов функций: `fn_name(a, b)`.
- Объявление переменных только через `new`.

Примеры:
```skadi
new Int x = 10
new i32 List nums = [1, 2, 3]
x++
if x > 0 {
    output(x)
}
```

## 3. Типы (v1-срез)

Рекомендуемый стиль имен:
- фиксированная разрядность: `i8/i16/i32/i64`, `u8/u16/u32/u64`, `f32/f64`,
- высокоуровневые: `Int`, `Float`, `Bool`, `Char`, `Text`, `Path`, `List`,
- `bool`/`char` допустимы как алиасы совместимости.

Поддерживаются:
- числа, bool, char, строки,
- `Text`,
- типизированные `List` через форму `new <type> List name = [...]`.

## 4. Управление потоком

### 4.1 Условия
```skadi
if cond {
    ...
} else {
    ...
}
```

### 4.2 when
```skadi
when code {
    is 1 {
        output("one")
    }
    else {
        output("other")
    }
}
```

### 4.3 Циклы
```skadi
for item in items {
    if item == 0 {
        continue
    }
}

iterate items as item {
    if item < 0 {
        break
    }
}
```

`pass` разрешен как no-op инструкция.

## 5. Ошибки и on error

Текущий контракт:
- `on error` поддержан для:
  - вызовов `danger fn`,
  - `List.pop()`.
- Для других выражений `on error` пока не универсален.

Пример:
```skadi
new Int v = parse_num(text) on error {
    v = 0
}
```

## 6. Встроенные возможности ядра (v1)

I/O:
- `output(x)`
- `input(prompt)`
- `read(path)`
- `write(path, data)`
- `args()`

FS:
- `fs.list(path)`
- `fs.is_dir(path)`
- `fs.join(a, b)`

Text:
- `len(text)`
- `contains(text, sub)`
- `find(text, sub)`
- `slice(text, start, end)`
- `text[i]`

List:
- литералы `[...]`,
- `push`,
- `pop() on error { ... }`,
- `len(list)`,
- `list[i]`.

## 7. Импорты и multi-file (V1-контракт)

Каноническая форма:
```skadi
import "./relative_path.skd"
```

На первой волне v1:
- поддерживается только path-import,
- `import module_name` и `as alias` отложены,
- resolver делает дедупликацию, детект циклов и детерминированный порядок склейки.

## 8. Что еще ограничено

- Полная memory-модель чанков пока не включена в runtime.
- Полный lowering `struct`/methods в C еще дополняется.
- `on event`, `run/wait/Link` и часть advanced-runtime фич остаются в backlog.

## 9. Минимальный пример

```skadi
label ErrorCode {
    Ok
    ZeroDivision
}

danger fn safe_div(Int a, Int b) Int {
    if b == 0 {
        return error ZeroDivision
    }
    return a div b
}

fn main() Int {
    new Int x = safe_div(10, 2) on error {
        x = 0
    }
    output(x)
    return 0
}
```
