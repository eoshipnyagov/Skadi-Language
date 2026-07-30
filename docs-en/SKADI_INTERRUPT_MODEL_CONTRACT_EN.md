# `on interrupt` Contract

Status: **Accepted design, host periodic MVP implemented**.

```skadi
Channel(Int) ticks = channel(8)
Interrupt timer = interrupts.periodic(10ms)

on interrupt timer {
    ticks.try_send(1)
}
```

The handler runs in a strict interrupt context. Allocation, blocking, I/O,
sleeping, task/resource management, and ordinary calls are forbidden.
`Channel.try_send` is the first supported bridge to normal context. Windows and
POSIX simulate the source with a periodic host thread; hardware IRQ backends
remain future work.
