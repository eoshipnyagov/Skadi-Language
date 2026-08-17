# Struct, Text, and List

```skadi
struct Counter {
    Int value
    hide Int limit

    fn bump(Int step) returns Int {
        my.value = my.value + step
        return my.value
    }
}

new Counter counter = {value = 0, limit = 10}
```

`hide` restricts field access to methods of the same struct. Struct literals
support field punning: `{x, y}`.

`label` is a numeric nominal type with explicit unique discriminants. `tag` is
a symbolic nominal type without a user-visible numeric contract.

```skadi
label ExitCode {
    Success = 0
    InvalidInput = 64
}

tag Direction {
    North
    South
}

new Direction direction = Direction.North
when direction {
    is North { output("north") }
    is South { output("south") }
}
```

Both forms work as value, parameter, and return types and do not mix with
`Int` or another nominal type. `ErrorCode` uses the same label machinery with
the additional requirement that its first variant is `Ok = 0`.

Text builtins are `len`, `contains`, `find`, `slice`, and `concat`. The current
Text runtime is byte-oriented.

```skadi
new Int List values = [1, 2, 3]
values.push(5)

new Int last = 0
last = values.pop() on error {
    output("empty")
}
```

List type syntax is `ElementType List`, never `List(ElementType)`.
