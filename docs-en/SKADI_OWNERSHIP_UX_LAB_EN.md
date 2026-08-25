# Ownership UX Lab: Own, Lend, Give

Date: 2026-07-30
Status: **accepted UX contract / bounded ownership engine implemented**

This document records the UX of the implemented bounded ownership engine. It
does not promise a full lifetime calculus.

Related contracts:

- [Borrow access and resources](edit-borrow-resource-contract.en.md)
- [Memory MVP](memory-model-mvp.en.md)
- [Task MVP](task-model-mvp.en.md)
- [Channel lifecycle](channel-lifecycle-contract.en.md)

## 1. Explanation without compiler terminology

A program has things and their owners.

1. **Own.** If I create a resource, it is mine and I am responsible for it.
2. **Lend for observation.** If a resource cannot be copied, `view`
   lets a function inspect the original without changing or keeping it.
3. **Lend for mutation.** `edit` lets a function temporarily modify the
   original. It belongs to the owner again after the call.
4. **Give.** `move` transfers ownership. The old owner can no longer use
   the resource.
5. **Leave the room.** When a scope ends, every resource it still owns is
   released automatically.

The short formula is:

```text
a value can be copied
a resource can be lent or given
a borrowed resource always returns after the call
a given resource no longer belongs to its old name
```

If an ordinary function requires more theory than this, the design is too
complicated.

## 2. User-visible words

| Intent | Candidate form | Status |
|---|---|---|
| Create an owner | `Canvas frame = canvas(...)` | Implemented |
| Read a copyable value | `fn show(Int value)` | Implemented / preferred |
| Inspect a resource | `view Canvas frame` | Implemented |
| Mutate temporarily | `edit Canvas frame` | Implemented |
| Pass a read-only borrow | `inspect(view frame)` | Implemented |
| Pass a mutable borrow | `draw(edit frame)` | Implemented |
| Transfer ownership | `consume(move frame)` | Implemented |
| Return ownership | `return move frame` | Implemented |
| Release automatically | scope exit | Implemented per resource, not unified |

The owner remains an ordinary name. `view`, `edit`, and `move` must remain
visible at the call site so an ownership boundary is never hidden in a signature.

## 3. Ten-scenario lab

### Scenario 1. Ordinary values copy

Status: **Implemented**.

```skadi
new Int original = 10
new Int copy = original
copy = copy + 1

output(original)
output(copy)
```

`original` remains `10`. Ownership must not add ceremony to simple value types.

### Scenario 2. Read a copyable value by value

Status: **Implemented**.

```skadi
fn show(Int value) {
    output(value)
}

constant Int answer = 42
show(answer)
```

The function receives an independent value. Assigning its local `value` does
not change `answer`.

`view Int` is technically accepted by the current bounded slice, but
it provides no useful user-visible behavior for `Int` and should not be
canonical style. The compiler may optimize value passing without changing
source semantics.

### Scenario 3. Temporarily mutate the original

Status: **Implemented**.

```skadi
fn increment(edit Int value) {
    value = value + 1
}

new Int count = 1
increment(edit count)
output(count)
```

The borrow ends after the synchronous call and the owner can use `count` again.

### Scenario 4. A resource lives until scope exit

Status: **Implemented per resource**.

```skadi
fn render_preview() {
    Canvas frame = canvas(64, 48)
    frame.clear(Color.terminal_black)
    frame.circle(
        {x = 32.0, y = 24.0},
        12.0,
        Color.terminal_bright_yellow
    )
}
```

`frame` is released automatically, including on an early return. Cleanup and
ownership checks are currently implemented separately for different resource
types rather than by one state machine.

### Scenario 5. Borrowed resource observation and mutation

Status: **Implemented for Canvas**.

```skadi
fn paint(edit Canvas frame) {
    frame.clear(Color.terminal_blue)
}

fn fingerprint(view Canvas frame) returns Int {
    return frame.checksum()
}

Canvas frame = canvas(64, 48)
paint(edit frame)
new Int checksum = fingerprint(view frame)
```

`fingerprint` cannot call `clear`, while `paint` can. Neither function can keep
the Canvas after returning.

### Scenario 6. Lifecycle across branches

Status: **Implemented for Window and Channel**.

```skadi
Canvas frame = canvas(320, 200)
Window window = windows.open("Preview", 320, 200)
new Bool should_close = true

if should_close {
    window.close()
}

window.present(edit frame) on error {
    pass
}
```

After the `if`, `window` is `maybe closed`. The operation is accepted because
the possible failure is handled with `on error`. Without the handler, the
compiler reports the lifecycle state and suggests adding `on error` or
restructuring control flow.

An operation on a definitely closed resource remains legal with `on error`, but
the compiler warns that execution is guaranteed to enter the handler. Closed
state is recoverable; ownership lost through `move` is not.

### Scenario 7. A Task stays owned until `wait`

Status: **Implemented**.

```skadi
fn measure() returns Int {
    return 42
}

Task(Int) work = run measure()
new Int result = wait work
output(result)
```

A scope cannot end with a live Task. `wait` consumes the handle and a second
`wait work` is an error.

### Scenario 8. Give a resource to a function

Status: **Implemented**.

```skadi
fn finish(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(64, 48)
finish(move frame)
output(frame.checksum())
```

The final line should report:

```text
resource 'frame' was moved to 'finish' and is no longer available
```

Both sides expose `move`: the function accepts ownership and the call site
shows that the old name becomes unavailable.

### Scenario 9. A factory returns a resource

Status: **Implemented**.

```skadi
fn make_frame(Int width, Int height) returns Canvas {
    Canvas frame = canvas(width, height)
    frame.clear(Color.terminal_black)
    return move frame
}

Canvas frame = make_frame(320, 200)
```

Ownership moves to the caller. The assignment needs no extra `move`: the
function result is a fresh temporary owner and leaves no usable old name.

### Scenario 10. Control flow cannot hide a partial move

Status: **Implemented**.

```skadi
fn finish(move Canvas frame) {
    output(frame.checksum())
}

Canvas frame = canvas(64, 48)
new Bool hand_off = true

if hand_off {
    finish(move frame)
}

output(frame.checksum())
```

The diagnostic should explain that `frame` is not available on every path.
Obvious repairs are to use it entirely inside each branch, move it after the
branch, or create a separate resource for the receiver.

The same rule applies to loops. A resource moved in one iteration cannot
silently reappear in the next.

## 4. The twelve-year-old test

If a variable contains an ordinary copyable value, a function can receive a
copy. Changing that copy does not change the original variable.

If a variable owns a resource, `view` lets a function inspect it. Multiple
read-only views may exist at the same time. `edit` lets one function
temporarily use and modify the original. The owner regains access after the
synchronous call.

`move` gives the resource to another owner. The old name can no longer
be used. A closed resource is different from a moved resource: its variable
still exists, so a failed operation can be handled explicitly with `on error`.

When an owner leaves its scope without transferring the resource, the program
releases it automatically.

Users should not need to understand lifetime parameters, reference counting,
allocator internals, generated C pointers, borrow-checker implementation, or
platform handles.

Result:

- passing copyable values by value passes the explainability test;
- `edit` for caller mutation and `view` for resource observation
  pass the explainability test;
- scope cleanup passes;
- explicit `move` is the natural ownership-transfer verb;
- implicit moves of named resources would be harder to explain;
- compiler internals should unify the special rules for `Memory`, `Task`,
  `Channel`, `Canvas`, and `Window` without adding user-facing syntax.

## 5. Bounded ownership-engine invariants

These are the current bounded contract:

1. Every resource has exactly one owner.
2. A closable resource is `open`, `maybe closed`, or `closed`; ownership
   separately tracks `owned` and `moved`.
3. A borrow lasts for one synchronous call.
4. `view` rejects logically mutating operations.
5. `view` and `edit` cannot cross `run`, be stored, or be returned.
6. Operations on `maybe closed` or `closed` require `on error`; a definitely
   closed operation also produces a warning.
7. `move` ends access through the source name and cannot be recovered with
   `on error`.
8. Branch merging checks state on every continuing path.
9. Loops do not restore moved resources implicitly.
10. The current owner performs cleanup exactly once in reverse acquisition order.
11. Diagnostics name the resource, operation, transfer point, and practical fix.

Canonical UX rule:

```text
copyable value: read by value, mutate the caller through edit
resource: read through view, mutate through edit, give through move
```

## 6. What the lab revealed

Already coherent:

- value copying;
- immutable and mutable synchronous borrowing;
- Task consumption through `wait`;
- deterministic cleanup for individual resource types;
- rejection of borrows across task boundaries;
- shared `open / maybe closed / closed` lifecycle tracking for Window and Channel;
- `on error` recovery without accessing a released OS handle.

Remaining limits:

- `Memory` remains a non-movable region capability;
- `Task` keeps its dedicated consume-through-`wait` lifecycle;
- borrows remain call-scoped rather than first-class references;
- new resource types need explicit integration with ownership and cleanup.

The accepted surface uses explicit `move` in parameters, call sites, bindings,
and `return`.
