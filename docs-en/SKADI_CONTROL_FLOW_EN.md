# Control Flow

```skadi
if temperature > 30 {
    output("hot")
} else {
    output("normal")
}

when status {
    is 0 {
        output("ok")
    }
    else {
        output("unknown")
    }
}
```

Canonical collection iteration:

```skadi
iterate values as value {
    output(value)
}
```

`for value in values`, `while condition`, and `loop` also work. Use `break`,
`continue`, and `pass` where appropriate.

C-style `for (init; condition; update)` is compatibility syntax and is rejected
by semantic analysis.
