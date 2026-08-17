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

Use `constant` for immutable bindings; `fixed` and `const` are ordinary names,
not declaration modifiers. Avoid contextual `allow` outside Memory, C-style loops, legacy return syntax,
invented I/O error forms, and mutable/capability payload across channels.

Run `check -> format -> build -> run` as the normal project discipline. Skadi
style favors visible cost, ownership, recovery, and dependencies over clever
shortcuts.
