# Skadi v1.0.0-rc1: Release Notes (Draft)

Date: 2026-05-27

## Что это за релиз

`v1.0.0-rc1` — кандидат в первый стабильный релиз транспилятора `Skadi -> C`.
Фокус релиза: надежность pipeline, тестовая защита от регрессий и предсказуемая CLI-диагностика.

## Что нового

1. Языковой pipeline:
- поддержаны `break/continue/pass` и statement-only `i++/i--`.

2. Надежность codegen:
- существенно расширен e2e-набор (`tests/codegen_e2e.rs`),
- добавлены стресс-сценарии на большие циклы, list/text/struct и длинные `when`.

3. Multi-file/import:
- усилены e2e/negative кейсы для import-графа (deep chain, wide diamond, cycle/invalid forms).

4. Диагностики:
- нормализован pipeline-формат ошибок (`code + stage + hint`),
- добавлены контрактные тесты стабильности диагностик.

5. CI:
- отдельный required job `codegen-e2e`,
- optional sanitizer job с явным логированием skip/pass.

## Ограничения текущего релиза

- модульный контракт v1: только `import "./relative_path.skd"`,
- расширенная модульная модель (alias/module-name/visibility) отложена,
- math/vector core и часть runtime-фич перенесены в `v1.x`.

## Как проверить локально

```bash
cargo test -q
cargo clippy --all-targets --all-features
cargo clippy --manifest-path tools/skadi-cli/Cargo.toml --all-targets --all-features
```

CLI smoke:

```bash
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- doctor
cargo run --manifest-path tools/skadi-cli/Cargo.toml -- new console demo
cd demo
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- check
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- build
cargo run --manifest-path ../tools/skadi-cli/Cargo.toml -- run
```
