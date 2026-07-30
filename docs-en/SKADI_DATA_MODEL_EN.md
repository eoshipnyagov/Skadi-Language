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

Only the special `label ErrorCode` has a complete runtime contract, and it must
start with `Ok`. General `label Status { ... }` declarations are visible to the
parser/module layer but do not yet have a complete value/type/C-lowering
contract.

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
