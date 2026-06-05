param(
  [switch]$Serve
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ($Serve) {
  python scripts/sync_docs_site.py
  python -m mkdocs serve
  exit $LASTEXITCODE
}

python scripts/sync_docs_site.py
python -m mkdocs build --strict
python scripts/postprocess_docs_preview.py

$indexPath = Join-Path $PSScriptRoot "..\site\index.html"
Start-Process $indexPath
