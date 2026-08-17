# I/O and Filesystem

The current I/O model is small and synchronous.

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

`read`, `write`, and filesystem functions are regular builtins, not danger
calls. Streams, async I/O, typed file errors, and resource handles are future
design work.
