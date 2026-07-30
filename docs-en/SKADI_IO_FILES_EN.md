# I/O and Filesystem

The current I/O model is small and synchronous.

```skadi
new Text List cli_args = args()
new Text name = input("Name: ")
output(concat("Hello, ", name))

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

`read`, `write`, and filesystem functions are regular builtins, not danger
calls. Streams, async I/O, typed file errors, and resource handles are future
design work.
