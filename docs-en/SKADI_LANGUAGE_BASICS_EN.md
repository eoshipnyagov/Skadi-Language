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

`Int` follows the target profile and may be pinned through
`[numeric] int = "target|i8|i16|i32|i64"` in `skadi.toml`; desktop targets
currently default to `i32`. Use fixed-width types for ABI, FFI, protocols, and
bit operations. `Float`/`f32` are 32-bit and `f64` is explicit. Integer literals
support `0b`, `0o`, `0x`, and `_` separators.

Operators include `+ - * / % ^`, `div`, `mod`, comparisons, and
`and/or/xor/not`. Symbolic `&&`, `||`, and `!` also work.

```skadi
new Int second = values[1]
new Char first = title[0]
new Bool valid = count > 0 and ready
```

Text indexing is byte-oriented and returns an ASCII `Char`. Out-of-range
indexing is fail-soft. Immutable bindings use `constant`:

```skadi
constant Int max_retries = 3
```

`fixed` and `const` are ordinary identifiers, not keyword aliases for
`constant`.
