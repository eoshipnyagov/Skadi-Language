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

Для короткой операции над целым файлом остаются простые builtins:

```skadi
new Path path = "notes.txt"
new Int written = write(path, "hello")
new Text content = read(path)
output(content)
```

`read` и `write` являются обычными builtins текущего runtime, а не `danger fn`.
Не добавляйте к ним `on error`: такой формы сейчас нет.

Если нужен явный ресурс, раннее закрытие или несколько операций над одним
дескриптором, используйте built-in `File`:

```skadi
fn copy_preview(Path path) returns Int {
    new File file = fs.open(path, FileMode.Read) on error {
        output("open failed")
        return -1
    }
    new Text content = file.read_all() on error {
        output("read failed")
        return -2
    }
    file.close() on error {
        output("close failed")
        return -3
    }
    output(content)
    return 0
}
```

Режимы: `FileMode.Read`, `FileMode.Write`, `FileMode.Append` и
`FileMode.ReadWrite`. `fs.open`, `File.read_all`, `File.write` и `File.close`
являются fallible operations и требуют `on error`.

- `Read` открывает существующий файл только для чтения;
- `Write` создает файл или очищает существующий;
- `Append` создает файл при необходимости и пишет в конец;
- `ReadWrite` открывает существующий файл для чтения и записи без очистки.

`read_all` читает файл целиком от начала и оставляет курсор в конце файла.

`File` имеет одного владельца. Его можно передать через `view`, `edit` или
`move`, но `read_all` и `write` меняют состояние курсора и поэтому доступны
только owner или `edit File`. Закрыть ресурс может только owner. Если явный
`close` не вызван, открытый дескриптор автоматически закрывается при выходе из
scope; явный `close` нужен для раннего освобождения и проверки ошибки закрытия.

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
| `fs.open(path, FileMode)` | `File`; только typed fallible declaration |
| `file.read_all()` | `Text`; требует `on error` |
| `file.write(text)` | без результата; требует `on error` |
| `file.close()` | без результата; требует `on error` |
| `fs.list(path)` | `Text List` |
| `fs.join(left, right)` | `Text` |
| `fs.is_dir(path)` | `Bool` |

Streams, async I/O, seek и typed file error values пока не реализованы. Текущий
`File` является небольшим синхронным resource API, а не stream abstraction.
