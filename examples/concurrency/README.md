# Concurrency examples

- `01_five_workers.skd` starts five native tasks before joining them and collects
  results through a bounded channel. The expected output is `55`.
- `02_restart_task.skd` creates a fresh result-bearing task handle in every loop
  iteration. The expected output is `15`.
- `03_cancel_blocked_channel.skd` stops a task waiting on an empty channel and
  verifies that cancellation neither closes the channel nor removes later data.
- `04_timed_channel.skd` demonstrates bounded `send_for`/`receive_for` waits and
  the contextual `timed_out` branch without exposing runtime status codes.
  The expected output is `1`, `41`, and `7` on separate lines.
- `05_timed_task_wait.skd` demonstrates path-sensitive timed Task ownership:
  success consumes the handle, while timeout keeps it live for explicit
  `stop`/`wait` cleanup. It prints a timeout message, `41`, and `7`.

These files are executable examples for the experimental `v1.2` Task/Channel
runtime MVP. The full contract is documented in
`docs/SKADI_CONCURRENCY_GUIDE_RU.md`.

With an installed CLI, run one standalone example without a `Skadi.toml`
project or Cargo:

```powershell
skadi-cli quick-run examples/concurrency/01_five_workers.skd
```

`quick-run` removes its temporary C and executable artifacts after the program
finishes. Regular projects should use `skadi-cli build/run`.
