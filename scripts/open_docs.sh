#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--serve" ]]; then
  python scripts/sync_docs_site.py
  python -m mkdocs serve
  exit 0
fi

python scripts/sync_docs_site.py
python -m mkdocs build --strict
python scripts/postprocess_docs_preview.py

if command -v open >/dev/null 2>&1; then
  open site/index.html
elif command -v xdg-open >/dev/null 2>&1; then
  xdg-open site/index.html
fi
