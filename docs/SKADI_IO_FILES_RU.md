# Ввод, вывод и файловая система

Текущий I/O слой намеренно небольшой и синхронный. Он не является stream API.

## Консоль и аргументы

```skadi
new Text List cli_args = args()
output("Skadi")

new Text name = input("Name: ")
output("Hello, ", name)
```

`output` принимает один или несколько аргументов типов `Int`, `Float`, `Bool`,
`Char` и `Text`. Значения печатаются подряд без скрытых пробелов и получают один
перевод строки после последнего аргумента. Нужные разделители задаются явно:

```skadi
output("count: ", count, ", ready: ", ready)
```

Такой вывод не создаёт промежуточный `Text`. Для получения текстового значения
по-прежнему используйте `concat` там, где он подходит. `args()` возвращает
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
        output("dir: ", name)
    } else {
        output("file: ", name)
    }
}
```

| Builtin | Возвращает |
|---|---|
| `args()` | `Text List` |
| `input(prompt)` | `Text` |
| `output(value, ...)` | `Int`; минимум один printable-аргумент |
| `read(path)` | `Text` |
| `write(path, text)` | `Int` |
| `fs.list(path)` | `Text List` |
| `fs.join(left, right)` | `Text` |
| `fs.is_dir(path)` | `Bool` |

Streams, async I/O, typed file errors и resource handles остаются отдельным
будущим design track.
