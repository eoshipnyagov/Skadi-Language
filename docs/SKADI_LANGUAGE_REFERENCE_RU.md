# Skadi: Справочник языка (RU)

Актуальный справочник по реализованному подмножеству Skadi в этом репозитории.

Роль этого документа: быть справочником по синтаксису, типам, builtins и
контрактам языка. Для первого знакомства и пошагового старта лучше начинать с
[Начало работы](getting-started.md).

[Начало работы](getting-started.md)  
[Статус синтаксиса](syntax-status.md)

## 1. Общая модель

Текущий практический контур проекта:

```text
Skadi source -> lexer -> parser -> semantic -> C codegen -> C compiler
```

Для `v1.1` основная цель - стабильный и тестируемый `Skadi -> C` pipeline,
а не финальный native backend.

## 2. Топ-уровневые конструкции

Поддерживаются:

- `fn`
- `danger fn`
- `label`
- `struct`
- обычные executable statements на top level
- `on interrupt ... { ... }` как parse-level конструкция

## 3. Объявления

### Без явного типа

```skadi
new x = 10
new title = "Skadi"
```

### С явным типом

```skadi
new Int count = 10
new Float ratio = 1.5
new Text name = "Alice"
new Path root = "."
new i32 List xs = [1, 2, 3]
```

### Присваивание

```skadi
count = count + 1
```

### Инкремент / декремент

```skadi
count++
count--
```

Они разрешены как statements, а не как expressions.

## 4. Типы

### Чаще всего используемые

- `Int`
- `Float`
- `Bool`
- `Char`
- `Text`
- `Path`

### Fixed-width типы

- `i8`, `i16`, `i32`, `i64`
- `u8`, `u16`, `u32`, `u64`
- `f32`, `f64`

### Совместимость имен

- `bool` и `char` принимаются как совместимые алиасы;
- в каноническом стиле предпочтительны `Bool` и `Char`.

### Приведение

Текущий зафиксированный случай неявного widening:

- `Int -> Float`

## 5. Литералы

Поддерживаются:

- integer literals
- float literals
- `true` / `false`
- string literals
- list literals
- struct literals

Примеры:

```skadi
new Int a = 10
new Float b = 2.5
new Bool ok = true
new Text t = "hello"
new i32 List xs = [1, 2, 3]
new Point p = {x = 10, y = 20}
```

Field punning:

```skadi
new Int value = 7
new Text status = "ok"
new Result r = {value, status}
```

## 6. Выражения

Поддерживаются:

- бинарные арифметические операции;
- сравнение;
- логические `and` / `or`;
- группировка через `(...)`;
- function calls;
- method calls;
- field access;
- indexing.

Примеры:

```skadi
new Int total = (a + b) * 2
new Bool ok = (total > 0) and ready
new Int second = xs[1]
new Char first = t[0]
new Int next = counter.bump(2)
```

## 7. Функции

### Обычная функция

```skadi
fn add(Int a, Int b) Int {
    return a + b
}
```

### `danger fn`

```skadi
danger fn safe_div(Int a, Int b) Int {
    if b == 0 {
        return error ZeroDivision
    }

    return a / b
}
```

### Параметры и возврат

Поддерживаются:

- typed params;
- typed return;
- вызов функции внутри выражения;
- проверка количества и типов аргументов.

## 8. Возврат и error flow

### `return`

```skadi
return value
return
```

### `return error`

```skadi
return error ZeroDivision
```

Работает только внутри `danger fn` и только при корректном `label ErrorCode`.

## 9. `label ErrorCode`

Пример:

```skadi
label ErrorCode {
    Ok
    ZeroDivision
    FileError
}
```

Текущий semantic contract:

- первый вариант должен быть `Ok`
- `return error X` требует, чтобы `X` существовал в `ErrorCode`

## 10. Управляющие конструкции

### `if / else`

```skadi
if total > 0 {
    output("positive")
} else {
    output("other")
}
```

### `while`

```skadi
while i < 10 {
    i++
}
```

### `loop`

```skadi
loop {
    break
}
```

### `for item in collection`

```skadi
for item in xs {
    output(item)
}
```

### `iterate collection as item`

```skadi
iterate xs as item {
    output(item)
}
```

`iterate ... as ...` - рекомендуемая витринная форма.  
`for ... in ...` поддерживается как совместимая и привычная форма.

### Legacy C-style `for`

```skadi
for (i = 0; i < 10; i++) {
    output(i)
}
```

Поддерживается, но не является каноническим стилем.

### `when / is / else`

```skadi
when mode {
    is 1 {
        output("one")
    }
    is 2, 3 {
        output("two or three")
    }
    else {
        output("other")
    }
}
```

### `break`, `continue`, `pass`

```skadi
break
continue
pass
```

`break` и `continue` разрешены только внутри циклов.

## 11. `on error`

### С присваиванием

```skadi
new Int value = safe_div(10, 2) on error {
    output("failed")
    return
}
```

### Без присваивания

```skadi
safe_div(10, 0) on error {
    output("failed")
}
```

Правило:

- `on error` можно использовать только на вызовах, которые semantic layer считает `danger`.

## 12. Struct и методы

```skadi
struct Counter {
    Int value

    fn inc(Int delta) Int {
        my.value = my.value + delta
        return my.value
    }
}

new Counter c = {value = 1}
new Int next = c.inc(2)
output(c.value)
```

Поддерживаются:

- поля структуры;
- методы;
- `my.field` внутри метода;
- `obj.field`;
- `obj.method(...)`;
- list of structs;
- методы на элементах списка после извлечения/итерации.

## 13. `Text`, `Path`, `List`

### `Text`

```skadi
new Text t = "weather"
new Int n = len(t)
new Char c = t[0]
```

### `Path`

```skadi
new Path root = "."
new Path full = fs.join(root, "src")
```

### `List`

```skadi
new i32 List xs = [1, 2, 3]
xs.push(4)
new i32 first = xs[0]
new Int size = len(xs)
```

### Индексация

```skadi
new i32 value = xs[1]
new Char c = t[0]
```

Текущий runtime-контракт `v1`:

- `List` index вне диапазона -> fail-soft default value;
- `Text` index вне диапазона -> `'\0'`;
- `on error` на индексации нет.

## 14. Builtins

### Core collection / text

- `len(x)`
- `contains(text, needle)`
- `find(text, needle)`
- `slice(text, start, end)`
- `concat(a, b)`

### Filesystem

- `fs.list(path)`
- `fs.is_dir(path)`
- `fs.join(a, b)`

### I/O

- `args()`
- `output(value)`
- `input(prompt)`
- `read(path)`
- `write(path, text)`

### Math

- `abs(x)`
- `min(a, b)`
- `max(a, b)`
- `clamp(x, lo, hi)`
- `floor(x)`
- `ceil(x)`
- `round(x)`
- `sin(x)`
- `cos(x)`
- `atan2(y, x)`
- `sqrt(x)`
- `root(x, n)`
- `deg_to_rad(x)`
- `rad_to_deg(x)`

### Math constants

- `PI`
- `TAU`
- `E`
- `EPSILON`

## 15. I/O examples

```skadi
new Text List cli_args = args()
new Text answer = input("name: ")
new Text body = read("input.txt")
new Int ok = write("output.txt", body)
output(answer)
```

## 16. Filesystem examples

```skadi
new Path root = "."
new Path List entries = fs.list(root)

iterate entries as entry {
    new Path full = fs.join(root, entry)
    if fs.is_dir(full) {
        output(full)
    }
}
```

## 17. Math examples

```skadi
new Float heading_deg = 45.0
new Float heading_rad = deg_to_rad(heading_deg)
new Float dx = cos(heading_rad)
new Float dy = sin(heading_rad)
new Float distance = sqrt((dx * dx) + (dy * dy))
new Float restored_deg = rad_to_deg(atan2(dy, dx))
new Float bounded = clamp(restored_deg, 0.0, 90.0)
output(bounded)
```

## 18. `on interrupt`

Синтаксис уже принимается:

```skadi
on interrupt shutdown {
    output("cleanup")
}
```

Но полноценная семантика выполнения этого трека пока не считается завершённой в `v1.1`.

## 19. Диагностика

Пользовательские ошибки стараются быть нормализованными:

- `Lex error ...`
- `Parse error ... [SC-PARSE-*] ...`
- `Semantic error ... [SC-SEM-*] ...`

Это важная часть текущего контракта проекта.

## 20. Что пока не считать завершённой стабильной частью `v1.1`

Не стоит пока закладываться на это как на законченный слой `v1.1`:

- модульную систему / imports;
- concurrency primitives;
- memory-model features из future docs;
- visual core;
- systems additions tracks;
- завершённую семантику выполнения для `on interrupt` и родственных будущих hooks.

Если нужен явный список того, что мы сознательно не поддерживаем в `v1`,
смотри [v1 non-goals](../internal/v1-non-goals.md).
