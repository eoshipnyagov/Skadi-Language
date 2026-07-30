# Channel Lifecycle Contract

Status: **Accepted design, implemented host runtime contract**.

- only the owning binding may call `close`;
- queued messages remain readable after close;
- sending to a closed channel and receiving from a drained closed channel are
  fallible through `on error`;
- after conditional close, `send`, `receive`, and `close` require `on error`;
- handled operations on a definitely closed channel are accepted with a
  guaranteed-handler warning;
- `try_send` is non-blocking and returns `Bool`;
- channel storage is destroyed only after dependent tasks and interrupt handlers
  have stopped.
