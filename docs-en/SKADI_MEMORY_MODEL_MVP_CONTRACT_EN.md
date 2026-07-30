# Skadi Memory Model: Bounded Runtime Contract

Date: 2026-07-30  
Status: **implemented experimental runtime slice**

This contract describes executable behavior in the current parser, semantic
analyzer, C backend, and native runtime. The broader Memory Draft remains a
design reference rather than a syntax promise.

## Region forms

```skadi
Memory fixed_memory = memory(8kb)
Memory growing_memory = memory(8kb, allow grow, allow drop)

place in growing_memory {
    Memory child_memory = memory.child(1kb)
}

Memory static_memory = memory.static(4kb)
```

- `memory(size)` has fixed capacity by default.
- `allow grow` appends chunks and never reallocates existing region storage.
- `allow drop` records owner permission but never deletes live values
  automatically in the current runtime.
- `memory.child(size)` reserves fixed storage from the active parent region.
- `memory.static(size)` requires a positive compile-time `ByteSize` literal,
  cannot grow, and is allowed only at program root.

Creation may use trailing `on error`. Allocation overflow inside `place in`
activates that block's trailing `on error`.

## Lifetime

Values created in `place in` are associated with the active region. Semantic
checks reject:

- returning dynamic payload from a local region;
- storing shorter-lived payload in a longer-lived owner;
- use after `clear`;
- clearing a currently active region;
- nested placement in the same region;
- use of a child after its parent is cleared.

A child region cannot outlive its parent. Clearing a parent invalidates its
children. Dynamic and growth storage is released automatically when the owning
scope ends.

## Capability restrictions

`Memory` is a region capability rather than a regular value:

- it cannot be copied or reassigned;
- it cannot be returned, stored in a struct/List, or sent through a Channel;
- it is not transferred through `move`;
- it may be passed as an external capability to a synchronous function.

`Memory.clear()` resets the whole region at once. No per-object `free` or user
drop hook is part of this bounded slice.

## Not promised

- a general lifetime calculus or first-class references;
- automatic `allow drop` reclamation;
- an embedded allocator backend;
- arbitrary OS-resource management through Memory;
- transparent handling of every possible reference graph.
