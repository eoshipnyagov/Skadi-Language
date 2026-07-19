# Byte Sizes in Skadi

Status: experimental `ByteSize` MVP in the current `v1.2` line.

`ByteSize` is a nominal byte-count type. It is not an alias for `Int` and does
not mix with ordinary numbers implicitly.

## Quick example

```skadi
new ByteSize payload = 2kb
new ByteSize capacity = payload + 512b
new Bool enough = capacity >= 2kb
Memory scratch_memory = memory(capacity)
```

## Literals and representation

Supported joined integer literals are `b`, `kb`, `mb`, and `gb`. Multipliers are
binary: 1, 1024, 1024^2, and 1024^3 bytes. Literal conversion to the internal
signed `i64` byte representation is overflow-checked.

Fractional literals and standalone spaced forms are rejected. The legacy
`memory(8 mb)` spelling is accepted only inside `memory(...)` and formats to
`memory(8mb)`.

## Arithmetic

`ByteSize + ByteSize` and `ByteSize - ByteSize` return `ByteSize`. Comparisons
between two `ByteSize` values return `Bool`. Numeric mixing, multiplication,
division, and implicit conversions are rejected.

## Memory integration

`memory(...)` accepts a `ByteSize` expression, including variables and function
results. A zero or negative computed capacity enters the Memory allocation
failure path before conversion to the C runtime capacity type.

```skadi
fn with_overhead(ByteSize payload) returns ByteSize {
    return payload + 1kb
}

new ByteSize capacity = with_overhead(4kb)
Memory assets_memory = memory(capacity)
```

`ByteSize` is value-safe in structs, Lists, `Task(ByteSize)`, and
`Channel(ByteSize)`.

## MVP limits

Decimal/IEC unit families, fractional literals, scalar operations, automatic
`output` formatting, general dimensional algebra, and extended allocator
policies remain out of scope.

Compile-checked showcase: `benchmarks/bench_14_byte_size_budget.skd`.
