# Ownership and Resource Transfer

Skadi separates ordinary values from owning resources.

- Scalar and small value types are copied by value.
- `Canvas`, `Window`, `Interrupt`, owning `Channel`, and `external resource`
  handles have exactly one owner.
- `Memory` is a region capability and is not transferred through `move`.
- `Task` has a dedicated lifecycle and must be consumed through `wait`.

## Observe, mutate, or give

```skadi
fn fingerprint(view Canvas frame) returns Int {
    return frame.checksum()
}

fn paint(edit Canvas frame) {
    frame.clear(Color.terminal_blue)
}

fn consume(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(32, 24)
new Int before = fingerprint(view frame)
paint(edit frame)
consume(move frame)
```

- `view` is a read-only borrow for one synchronous call.
- `edit` is an exclusive mutable borrow for one call.
- `move` transfers ownership permanently and makes the old name unavailable.

The marker is visible in both the signature and call site because it changes
program behavior rather than merely guiding optimization.

## New bindings and factories

```skadi
Canvas first = canvas(32, 24)
Canvas second = move first
```

`second` is now the owner. A factory returns a resource explicitly:

```skadi
fn make_frame() returns Canvas {
    Canvas frame = canvas(32, 24)
    return move frame
}

Canvas frame = make_frame()
```

The function result is already a fresh owner value, so the assignment needs no
additional `move`.

## Control flow and cleanup

A resource moved in only one branch is unavailable on the merged path. A move
from a repeating loop body is rejected because the next iteration cannot
silently regain ownership.

Resources still owned when a scope ends are released automatically in reverse
acquisition order. After a move, only the new owner performs cleanup.

Opaque `external resource` is the exception: Skadi does not know the foreign
destructor. Its owner must be passed explicitly to a consuming `move` parameter
before scope exit, normally a danger close function with `on error`.

A fallible factory needs neither a nullable nor a dummy handle:
`new Resource value = create() on error { return ... }` creates the owner only
on success. `value` does not exist in the error handler, so that handler must
terminate the path with `return` or `return error`.

A closed resource differs from a moved resource: closed/maybe-closed operations
may be handled through `on error`, while use after move is a static error.

See the internal [Ownership UX Lab](../internal/ownership-ux-lab.en.md) for the
complete scenario matrix. Runnable example:
`examples/ownership/01_move_canvas_factory.skd`.
