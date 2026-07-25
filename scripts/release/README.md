# Skadi release tooling

`package_release.py` is the canonical archive builder used locally and in
GitHub Actions. It uses only the Python standard library.

```bash
python scripts/release/package_release.py verify-version \
  --version v1.2.0-rc.1

python scripts/release/package_release.py package \
  --binary target/x86_64-unknown-linux-musl/release/skadi-cli \
  --target x86_64-unknown-linux-musl \
  --version 1.2.0-rc.1 \
  --output-dir dist

python scripts/release/package_release.py checksums \
  --dist-dir dist \
  --output dist/SHA256SUMS
```

The requested version must match `[workspace.package].version`. Archive
timestamps, ownership, ordering, and executable modes are normalized so the
same inputs produce the same archive bytes.
