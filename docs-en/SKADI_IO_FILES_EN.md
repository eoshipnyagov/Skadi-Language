# I/O and Filesystem

The current I/O model is small and synchronous. It offers both convenient
whole-file builtins and an explicit owning `File` resource.

```skadi
new Text List cli_args = args()
new Text name = input("Name: ")
output("Hello, ", name)

new Path path = "notes.txt"
new Int written = write(path, "hello")
new Text content = read(path)
```

```skadi
new Text List names = fs.list(".")
iterate names as name {
    new Path full = fs.join(".", name)
    if fs.is_dir(full) {
        output(name)
    }
}
```

`output` accepts one or more `Int`, `Float`, `Bool`, `Char`, or `Text` values.
It writes them consecutively with no implicit spaces, then writes one newline:

```skadi
output("count: ", count, ", ready: ", ready)
```

Write spaces and punctuation explicitly. This path does not allocate an
intermediate `Text`.

`read` and `write` are regular whole-file builtins, not danger calls. Do not add
`on error` to them.

Use `File` when a handle, early close, or multiple operations are required:

```skadi
fn inspect(Path path) returns Int {
    new File file = fs.open(path, FileMode.Read) on error {
        return -1
    }
    new Text content = file.read_all() on error {
        return -2
    }
    file.close() on error {
        return -3
    }
    output(content)
    return 0
}
```

Available modes are `FileMode.Read`, `FileMode.Write`, `FileMode.Append`, and
`FileMode.ReadWrite`. `fs.open`, `read_all`, `write`, and `close` are fallible
and require `on error`.

- `Read` opens an existing file for reading only;
- `Write` creates a file or truncates an existing file;
- `Append` creates a file when needed and writes at the end;
- `ReadWrite` opens an existing file for reading and writing without truncation.

`read_all` reads the whole file from the beginning and leaves the cursor at the
end of the file.

`File` has one owner. `read_all` and `write` mutate cursor state, so they require
the owner or an `edit File` borrow; `view File` cannot call them. Only the owner
may close the handle. An open handle is closed automatically at scope exit, and
explicit `close` releases it earlier while exposing close failure.

Streams, async I/O, seek, and typed file error values are not implemented. The
current API is a bounded synchronous resource surface, not a stream abstraction.
