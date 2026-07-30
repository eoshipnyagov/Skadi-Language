# Ввод, вывод и файловая система

Текущий I/O слой намеренно небольшой и синхронный. Он не является stream API.

## Консоль и аргументы

```skadi
new Text List cli_args = args()
output("Skadi")

new Text name = input("Name: ")
output(concat("Hello, ", name))
```

`output` принимает `Int`, `Float`, `Bool`, `Char` и `Text`. `args()` возвращает
`Text List`.

## Файлы

```skadi
new Path path = "notes.txt"
new Int written = write(path, "hello")
new Text content = read(path)
output(content)
```

`read` и `write` являются обычными builtins текущего runtime, а не `danger fn`.
Не добавляйте к ним `on error`: такой формы сейчас нет.

## Пути и каталоги

```skadi
new Path root = "."
new Text List names = fs.list(root)

iterate names as name {
    new Path full = fs.join(root, name)
    if fs.is_dir(full) {
        output(concat("dir: ", name))
    } else {
        output(concat("file: ", name))
    }
}
```

| Builtin | Возвращает |
|---|---|
| `args()` | `Text List` |
| `input(prompt)` | `Text` |
| `output(value)` | `Int` |
| `read(path)` | `Text` |
| `write(path, text)` | `Int` |
| `fs.list(path)` | `Text List` |
| `fs.join(left, right)` | `Text` |
| `fs.is_dir(path)` | `Bool` |

Streams, async I/O, typed file errors и resource handles остаются отдельным
будущим design track.
