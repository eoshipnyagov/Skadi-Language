# Types, Literals, and Expressions

Skadi source uses `.skd`. Newlines terminate most statements; semicolons are not
required.

```skadi
// line comment
/* block comment */
new Int count = 42
new Float ratio = 0.5
new Bool ready = true
new Char marker = '\n'
new Text title = "Skadi"
new Int List values = [1, 2, 3]
```

Use explicit types for lists, structs, vectors, tasks, and channels. Scalar
values can be inferred. `Int -> Float` is the supported widening conversion.

Operators include `+ - * / % ^`, `div`, `mod`, comparisons, and
`and/or/xor/not`. Symbolic `&&`, `||`, and `!` also work.

```skadi
new Int second = values[1]
new Char first = title[0]
new Bool valid = count > 0 and ready
```

Text indexing is byte-oriented and returns an ASCII `Char`. Out-of-range
indexing is fail-soft. `fixed` and `const` are reserved but not implemented.
