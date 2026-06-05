#!/usr/bin/env bash
set -euo pipefail
python scripts/sync_docs_site.py
python -m mkdocs build --strict
python scripts/postprocess_docs_preview.py
