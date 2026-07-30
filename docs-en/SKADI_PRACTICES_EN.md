# Recommended Practices

Prefer canonical forms:

```skadi
new Bool ready = true
new Char marker = 'x'

iterate values as value {
    output(value)
}
```

Keep dependencies, return types, and recovery explicit:

```skadi
import "./geometry.skd"

fn origin_distance(geometry.Point point) returns Float {
    return geometry.length(point)
}
```

Avoid reserved `fixed/const/allow` forms, C-style loops, legacy return syntax,
invented I/O error forms, and mutable/capability payload across channels.

Run `check -> format -> build -> run` as the normal project discipline. Skadi
style favors visible cost, ownership, recovery, and dependencies over clever
shortcuts.
