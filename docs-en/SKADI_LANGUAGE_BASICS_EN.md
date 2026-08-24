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
Fixed-width integer variables do not convert implicitly; use the checked
`as_i8..as_i64` or `as_u8..as_u64` family with `on error`. Float inputs must be
finite, integral, and in range. `as_f32(...) on error` is checked narrowing;
`as_f64(...)` is safe widening. A fitting integer literal can be written
directly at the required fixed width.

`Int` follows the target profile and may be pinned through
`[numeric] int = "target|i8|i16|i32|i64"` in `skadi.toml`; desktop targets
currently default to `i32`. Use fixed-width types for ABI, FFI, protocols, and
bit operations. `Float`/`f32` are 32-bit; explicit `f64` preserves double
literal and math precision. Integer literals
support `0b`, `0o`, `0x`, and `_` separators.

Ordinary integer arithmetic checks overflow and division by zero and reports
runtime code `SC-RT-350`. Intentional overflow is explicit through
`wrapping_add/sub/mul/neg` and is available only for fixed-width integers.

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
