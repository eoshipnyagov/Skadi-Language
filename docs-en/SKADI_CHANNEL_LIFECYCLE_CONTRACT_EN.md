# Channel Lifecycle Contract

Status: **Accepted design, implemented host runtime contract**.

## States and operations

A bounded `Channel(T)` has two observable states:

```text
Open -> Closed
```

Closing prevents new messages but does not discard queued values. Only the
owning binding may call `close`; consumers can drain the buffer afterwards.

- `send(value)` blocks while an open channel is full;
- `try_send(value)` never blocks and returns `Bool`;
- `receive()` blocks while an open channel is empty;
- `send_for(value, Duration)` and `receive_for(Duration)` use one deadline;
- `close()` wakes waiting senders and receivers;
- closed or cancelled blocking operations recover through `on error`;
- timed operations always require `on error`;
- after conditional close, `send`, `receive`, and `close` require `on error`;
- a handled operation on a definitely closed channel receives a warning because
  its handler is guaranteed to run.

## Ownership

The channel owner must outlive every task and interrupt handler that uses it.
All dependent tasks must be waited and interrupt handlers stopped before channel
storage is destroyed. Parameters carry a borrowed channel capability; they
cannot close it.

## Cancellation and deadlines

`Closed`, `Cancelled`, and `TimedOut` are distinct runtime outcomes. They share
the language-level `on error` boundary:

- `stopping` identifies cancellation of the current task;
- `timed_out` identifies expiry of `send_for` or `receive_for`;
- the remaining handler path identifies a closed channel.

`stop` wakes only the blocked task being stopped and does not close or mutate the
channel. A cancelled send adds no message; a cancelled receive removes none. An
operation that already changed the queue before the stop remains successful.

Timed waits use one absolute deadline, so spurious wakeups do not extend it. An
immediately available operation wins over a zero timeout. After a wait has been
registered, cancellation wins over timeout; the runtime rechecks the queue and
closed state under the channel lock before returning `TimedOut`.

## Deferred

- timed `Task.wait`;
- `select` and `try_receive`;
- cancellation of file and arbitrary platform I/O;
- fairness guarantees and detached channel ownership.
