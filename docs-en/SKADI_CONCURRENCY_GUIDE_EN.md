# Concurrency in Skadi

Status: experimental `v1.2` systems surface available in the current compiler and
C backend.

Skadi uses tasks and message passing. Programs work with `Task` and `Channel(T)`,
not platform thread handles. On Windows and POSIX hosts, every running task is
currently mapped to a separate native thread.

## Quick example

```skadi
fn calculate(Int value) Int {
    return value * value
}

Task(Int) first_task = run calculate(3)
Task(Int) second_task = run calculate(4)
new Int first = wait first_task
new Int second = wait second_task
output(first + second)
```

Both tasks start before the first `wait`, so they may execute in parallel. `wait`
joins the task, takes its result, and releases its runtime resources.

## `Task` and `Task(T)`

A task without a result has type `Task`:

```skadi
fn save_report(Text report) {
    write("report.txt", report)
}

Task save_task = run save_report("ready")
wait save_task
```

A result-bearing task has type `Task(T)`:

```skadi
fn load_status() Text {
    return "ready"
}

Task(Text) status_task = run load_status()
new Text status = wait status_task
```

The function result and the `Task(T)` parameter must match. A `Task(T)` result is
received with a `wait` expression; plain `wait task` is used for a task without a
result.

A `danger fn` cannot be a task entry yet because transferring `ErrorCode` across
the task boundary has no dedicated contract.

## Running several tasks in parallel

There is no special `run 5` construct. Create five independent handles, start the
whole group, and only then wait for it:

```skadi
fn square_and_send(Int value, Channel(Int) results) {
    results.send(value * value)
}

Channel(Int) results = channel(2)
Task first_task = run square_and_send(1, results)
Task second_task = run square_and_send(2, results)
Task third_task = run square_and_send(3, results)
Task fourth_task = run square_and_send(4, results)
Task fifth_task = run square_and_send(5, results)

new Int total = 0
new Int received = 0
while received < 5 {
    new Int value = results.receive()
    total = total + value
    received++
}

wait first_task
wait second_task
wait third_task
wait fourth_task
wait fifth_task
output(total)
```

The desktop runtime imposes no artificial task-count limit, but each task consumes
an OS thread and stack. The practical limit depends on the operating system,
memory, and C toolchain. There is no thread pool or work-stealing scheduler yet,
so thousands of short tasks are not a recommended workload.

The start, completion, and ordinary `output` order across tasks is unspecified.
Programs should make their message protocol and final post-`wait` result
deterministic rather than depend on thread scheduling order.

Calling `run` and immediately `wait` before the next `run` makes the work
sequential.

## Channels

`Channel(T)` transfers value-safe messages between tasks:

```skadi
struct Reading {
    Int sensor_id
    Float value
}

Channel(Reading) readings = channel(8)
readings.send(reading)
new Reading next_reading = readings.receive()
```

`channel(N)` creates a bounded FIFO with explicit capacity:

- `N` must be greater than zero;
- `send` blocks while the buffer is full;
- `receive` blocks while the buffer is empty;
- messages from one channel are delivered in FIFO order;
- multiple producers and consumers may safely share one channel.

With multiple producers, FIFO reflects the actual order of successful `send`
operations, not source-code `run` order. Fairness between waiting threads is not
currently guaranteed.

The positive-capacity contract is checked at runtime. A violation terminates with
diagnostic `SC-RT-312`.

### Backpressure

A small buffer deliberately slows a producer when its consumer cannot keep up:

```skadi
Channel(Int) values = channel(1)
```

This is built-in backpressure. The current surface has no `try_send`,
`try_receive`, timeout, `select`, or `close`.

### Consumer completion

A channel does not close automatically. A consumer must know the message count or
receive an explicit protocol value that means completion. Waiting for a message
that no task will send causes a deadlock.

## Handle lifecycle

`Task` is a linear owning handle:

- every `run` result must be stored;
- every handle must be waited exactly once on every control-flow path;
- at most one `stop` is allowed before `wait`;
- a handle cannot be copied, reassigned, stored in a `List`/`struct`, or returned;
- leaving a scope with a live handle is semantic error `SC-SEM-070`;
- fire-and-forget and detached tasks are unsupported.

The compiler rejects an ignored `run worker()` because it could not guarantee
join and cleanup.

## Cooperative stop

`stop` publishes a stop request and `stopping` reads it inside the task entry:

```skadi
fn worker() {
    while not stopping {
        do_work()
    }
    cleanup()
}

Task worker_task = run worker()
stop worker_task
wait worker_task
```

`stop` does not forcibly kill a thread. The task must reach another `stopping`
check and exit by itself. `wait` remains mandatory after `stop`.

Blocking `receive`, `send`, and file I/O are not interrupted by a stop request.
Do not make a worker wait forever on an empty channel without a separate shutdown
message.

`stopping` is accepted only inside a function that the current program launches
with `run`. Repeating `stop` on one handle is a semantic error.

## Restarting and periodic work

A completed handle cannot be restarted. Start the same function again with a new
handle:

```skadi
new Int index = 0
new Int total = 0
while index < 5 {
    Task(Int) iteration_task = run calculate(index)
    new Int result = wait iteration_task
    total = total + result
    index++
}
```

A complete `run -> wait` lifecycle inside one iteration is supported. A handle
cannot be created outside a loop while its `wait` or `stop` depends on the number
of iterations, because the compiler could not prove unique cleanup.

This repeats work but does not define a time interval. The runtime has no stable
`sleep`/`delay`, timer API, or duration units yet. Busy waiting is discouraged. A
real periodic scheduler belongs to the future systems time contract.

## Values crossing a task boundary

Arguments to `run`, `Task(T)` results, and `Channel(T)` payloads are checked
recursively.

Allowed values include numeric scalars, `Bool`, `Char`, non-region-owned `Text`
and `Path`, and value-like structs whose fields are also safe. `Channel(T)` is a
special shared capability accepted as a task argument.

Forbidden values include `Memory`, `Task`, nested channels as messages, mutable
`List`, region-owned values from `place in`, and structs recursively containing
such capabilities or payloads.

Shared mutable memory is not the default model. A channel owns a shared queue,
while each transferred message is copied as a value-safe representation.

An owning `Channel(T) name = channel(N)` declaration must be outside loops and
`place in`. A borrowed Channel parameter may be used inside workers and loops.
The channel owner must outlive every task that receives it, and all such tasks
must be waited before the owner scope ends.

## Errors and deadlocks

The compiler prevents lost handles and unsafe data transfer, but it cannot prove
the absence of protocol deadlocks. Review these cases:

- a consumer expects more messages than producers send;
- a producer blocks on a full channel while the caller waits for that producer
  before receiving;
- two tasks wait for messages from each other;
- a worker remains in blocking `receive` after `stop`;
- a channel owner ends before a task using it.

Use the normal project workflow for diagnostics:

```text
skadi-cli check
skadi-cli build
skadi-cli run
```

Frontend failures use `SC-SEM-*`, runtime failures use `SC-RT-*`, and the CLI
classifies C compiler failures separately as toolchain failures.

If the runtime cannot create a native thread or synchronization object, the MVP
emits a coded runtime diagnostic and terminates the process. Recoverable
`run ... on error` is not available yet.

## Platform implementation

The current C backend uses:

- Windows: `CreateThread`, Win32 synchronization primitives, and thread-local task
  context;
- Linux and other supported POSIX hosts: `pthread_create`, `pthread_join`,
  mutex/condition variable, and thread-local context;
- one native thread per `Task`;
- mutexes and condition variables for `Channel`;
- thread-safe publication of `stop`.

This is a backend implementation detail, not a permanent language promise. Tasks
are concurrent but cannot physically run in parallel on a single-core system. On
a multi-core host the OS may execute them simultaneously. Skadi currently gives
no affinity, priority, stack-size, or real-time scheduling guarantees.

## ESP32 and microcontrollers

ESP32, ESP-IDF, FreeRTOS, and bare-metal targets are not yet supported by the
official CLI, backend, or CI. Generated POSIX C must not be treated as an ESP32
port even though ESP-IDF offers a pthread compatibility layer.

The preferred implementation path is:

1. make ESP-IDF the first target family, covering Xtensa and RISC-V chips;
2. add target profiles and toolchain discovery to `skadi-cli`;
3. validate a minimal backend through ESP-IDF pthread compatibility;
4. add a direct FreeRTOS backend for stack, priority, core affinity, and static
   allocation control;
5. replace unconditional heap allocation of task contexts and channel buffers
   with configurable static or region-backed storage;
6. add emulator smoke tests and hardware-in-the-loop tests.

Before that port, Skadi must define stack size, priority, core pinning, task
limits, Channel storage, and resource-exhaustion behavior. A single-core ESP32
provides concurrency without true parallel execution; dual-core chips may run
tasks in parallel when the scheduler permits it.

The current runtime is not hard real-time. Cooperative stop, dynamic native-thread
creation, and heap-backed channels do not provide bounded latency or deterministic
allocation.

## Current limitations

The following features are not available yet:

- task groups and structured-concurrency syntax;
- thread pool, work stealing, and async/await;
- detached tasks and hard kill;
- channel close, timeout, `select`, `try_send`, and `try_receive`;
- cancellation of blocking I/O or channel operations;
- timer, `sleep`/`delay`, and periodic scheduler;
- affinity, priority, and stack-size configuration;
- RTOS, ESP32, and bare-metal backends;
- hard real-time guarantees.

## Further reading

- [Language Reference](language-reference.md)
- [Showcase Programs](showcases.md)
- `benchmarks/bench_11_task_channel_pipeline.skd`
- `benchmarks/bench_12_systems_pipeline.skd`
- `examples/concurrency/01_five_workers.skd`
- `examples/concurrency/02_restart_task.skd`
