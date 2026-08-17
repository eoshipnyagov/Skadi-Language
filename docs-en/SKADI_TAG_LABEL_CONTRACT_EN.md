# `tag` and `label` Contract

Status: **Implemented stable contract**.

- `tag` declares a closed symbolic nominal set with no user-visible numeric ABI.
- `label` declares a closed numeric nominal set; every variant requires an
  explicit integer discriminant.
- `ErrorCode` must begin with `Ok = 0`.
- Both forms support qualification, local declarations, semantic type checking,
  and C lowering.
