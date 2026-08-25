# Memory

The `v1.2` memory model is an experimental region runtime with fixed, growing,
child, and static regions.

```skadi
Memory scratch = memory(8kb) on error {
    output("cannot create memory")
}

place in scratch {
    new Text preview = concat("frame-", "preview")
    output(preview)
} on error {
    output("budget exceeded")
}

scratch.clear()
```

`Memory` is a capability, not a regular copyable value. Dynamic payload created
inside `place in` is tied to the active region. Semantic checks reject dangerous
escapes and use after `clear`.

## Region policies

```skadi
Memory cache_memory = memory(64kb, allow grow, allow drop)

place in cache_memory {
    Memory scratch_memory = memory.child(8kb)
}

Memory device_memory = memory.static(4kb)
```

- `allow grow` appends stable-address chunks and never reallocates existing
  region storage.
- `allow drop` records explicit owner permission, but the current runtime never
  deletes live values implicitly.
- `memory.child` reserves fixed storage from the active parent region.
- `memory.static` requires a positive `ByteSize` literal, does not grow, and is
  allowed only at program root.
- clearing a parent also invalidates its child regions.

## Ownership transfer

`Canvas`, `Window`, `Interrupt`, and owning `Channel` handles have one owner.
They can be borrowed through `view`/`edit` or transferred explicitly:

```skadi
fn consume(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(32, 24)
consume(move frame)
```

Factories return a resource through `return move resource`. The old name is
statically unavailable after transfer, partial branch moves are diagnosed, and
moves from repeating loop bodies are rejected.

See [Ownership and Resource Transfer](ownership.en.md) for the complete user
contract.

An embedded allocator contract, automatic `allow drop` reclamation, and a
general lifetime calculus remain future work.
