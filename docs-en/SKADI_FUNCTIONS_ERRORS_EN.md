# Functions, Visibility, and Errors

```skadi
fn add(Int left, Int right) returns Int {
    return left + right
}
```

Parameters are typed and `returns Type` is canonical. `local fn` stays inside
the current file.

```skadi
label ErrorCode {
    Ok = 0
    InvalidInput = 1
}

danger fn require_positive(Int value) returns Int {
    if value <= 0 {
        return error InvalidInput
    }
    return value
}

new Int checked = 0
checked = require_positive(10) on error {
    output("invalid input")
}
```

`ErrorCode` must start with `Ok`. `return error` is valid only in `danger fn`,
and a danger call requires visible `on error` recovery.
