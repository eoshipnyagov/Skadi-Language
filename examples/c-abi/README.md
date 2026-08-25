# C ABI example

This project demonstrates the bounded Skadi C ABI with declarations isolated in
the `src/sensor.skd` binding module, a native C source, fixed-width arguments,
an `external danger fn` status/out adapter, scalar results, and call-scoped
`view Buffer(u8)` / `edit Buffer(u8)` access to a typed Skadi list. It also
passes an `external struct SensorReading` by value using ordinary C field order
and alignment.

From this directory:

```powershell
skadi-cli check
skadi-cli run
```

Expected program output:

```text
Calibrated reading: 400
Reading valid: true
Reading confidence: 0.75
Packet checksum: 60 -> 63
```

The public contract is documented in `docs/SKADI_C_ABI_RU.md` and
`docs-en/SKADI_C_ABI_EN.md`.
