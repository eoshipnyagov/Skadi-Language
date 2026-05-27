# Skadi Language Reference (v1 snapshot)

Date: 2026-05-27
Status: practical reference for currently implemented behavior.

## 1. Implemented pipeline

`Skadi source -> lexer -> parser -> semantic -> C codegen -> native compile (via skadi-cli)`

## 2. Core syntax

- One statement per line (no `;`).
- Blocks use `{ ... }`.
- Comments: `//` and `/* ... */`.
- Function call: `fn_name(a, b)`.
- Variable declaration uses `new`.

```skadi
new Int x = 10
new i32 List nums = [1, 2, 3]
x++
if x > 0 {
    output(x)
}
```

## 3. Types (v1)

Recommended type naming:
- fixed-size numeric: `i8/i16/i32/i64`, `u8/u16/u32/u64`, `f32/f64`
- readable aliases: `Int`, `Float`, `Bool`, `Char`, `Text`, `Path`, `List`
- compatibility aliases: `bool`, `char`

Supported now:
- numeric, bool, char, string/text
- `Text` operations
- typed lists via `new <type> List name = [...]`

## 4. Control flow

```skadi
if cond {
    ...
} else {
    ...
}

when code {
    is 1 { output("one") }
    else { output("other") }
}

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

`pass` is allowed as a no-op statement.

## 5. Errors and `on error`

Current v1 contract:
- `on error` is supported for:
  - `danger fn` calls
  - `List.pop()`
- it is not yet general-purpose for all expressions.

```skadi
new Int v = parse_num(text) on error {
    v = 0
}
```

## 6. Built-ins available in v1

I/O:
- `output(x)`, `input(prompt)`, `read(path)`, `write(path, data)`, `args()`

FS:
- `fs.list(path)`, `fs.is_dir(path)`, `fs.join(a, b)`

Text:
- `len(text)`, `contains(text, sub)`, `find(text, sub)`, `slice(text, start, end)`, `text[i]`

List:
- literals `[...]`, `push`, `pop() on error { ... }`, `len(list)`, `list[i]`

## 7. Imports and multi-file contract (v1)

Canonical form:

```skadi
import "./relative_path.skd"
```

In v1:
- only path import is supported
- `import module_name` and alias form (`as`) are deferred
- resolver provides deduplication, cycle detection, deterministic merge order

## 8. Known limits (still in backlog)

- full chunk memory model is not in runtime yet
- full `struct`/method lowering is still evolving
- `on event`, `run/wait/Link` and part of advanced runtime remain deferred

## 9. Style profile (recommended)

- Prefer `iterate <collection> as <item>` for showcase/user-facing examples.
- Keep fixed-size numeric types lowercase.
- Prefer `when` for CLI flag dispatch.
- Prefer explicit error labels (`ZeroDivision`, `BadInput`, ...).
