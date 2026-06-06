# Memory Large Examples

`01_log_analyzer.skd` is the first larger canonical Memory example.

What it demonstrates:

- local `scratch_memory` for temporary preview work;
- external `assets_memory` for region-owned returned data;
- `place in ... { ... } on error { ... }`;
- `clear()` in explicit recovery path;
- `Text`, `Text List`, loops, helper functions, and self-contained naming.

Input data:

- `examples/memory/large/data/sample_service.log`

Run from the repository root:

```powershell
cargo run -- --input examples/memory/large/01_log_analyzer.skd
```

Expected highlights:

- preview status `0`
- total lines `8`
- error lines `2`
- warning lines `2`
- todo lines `2`
- alert lines `6`
