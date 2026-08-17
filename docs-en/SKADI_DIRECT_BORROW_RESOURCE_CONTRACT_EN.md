# Borrow and Resource Contract

Status: **Accepted design, bounded borrow and ownership-transfer slice implemented**.

Readability and ownership-transfer scenarios are evaluated in the
[Ownership UX Lab](ownership-ux-lab.en.md).

- `direct T` is an explicit mutable call-scoped borrow;
- `view T` is an explicit read-only call-scoped borrow;
- the caller writes `direct value` or `view value`, so access is visible at
  both ends;
- an owning resource crosses a function boundary only through `move T` and
  `move value`;
- a factory returns ownership through `return move value`;
- borrows cannot be stored, returned, or moved across a task boundary;
- operations on `maybe closed` or `closed` resources require `on error`;
- a handled operation on a definitely closed resource is accepted with a
  warning;
- ownership lost through `move` and borrow violations are static errors and
  cannot be recovered through `on error`;
- `constant` remains the immutable binding keyword and is not part of borrow
  syntax;
- `view` restricts access through that parameter; it does not make the owned
  resource globally immutable;
- `view`, `direct`, and `move` remain fully reserved words rather than
  contextual keywords;
- `fixed` and `const` are ordinary identifiers, not declaration modifiers;
  the former `constant direct` form is not a language form.

`.close()` is not a universal resource interface. It is currently meaningful
for `Window` and `Channel`; future `File`, `Port`, and `Socket` handles may use
the same lifecycle engine. `Task`, `Interrupt`, `Memory`, and `Canvas` retain
their domain operations or automatic cleanup.
