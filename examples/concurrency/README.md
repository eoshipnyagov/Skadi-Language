# Concurrency examples

- `01_five_workers.skd` starts five native tasks before joining them and collects
  results through a bounded channel. The expected output is `55`.
- `02_restart_task.skd` creates a fresh result-bearing task handle in every loop
  iteration. The expected output is `15`.

These files are executable examples for the experimental `v1.2` Task/Channel
runtime MVP. The full contract is documented in
`docs/SKADI_CONCURRENCY_GUIDE_RU.md`.
