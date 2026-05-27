#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   bash scripts/leak_check_wsl.sh [path/to/source.c] [path/to/input.txt]
#
# Defaults:
#   source.c  -> examples/minidb_tui/build/minidb_tui.c
#   input.txt -> scripts/leak_check_minidb_input.txt

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_C="${1:-$ROOT_DIR/examples/minidb_tui/build/minidb_tui.c}"
INPUT_TXT="${2:-$ROOT_DIR/scripts/leak_check_minidb_input.txt}"
OUT_BIN="$ROOT_DIR/build/minidb_tui_leakcheck"
OUT_LOG="$ROOT_DIR/build/valgrind_minidb.log"

mkdir -p "$ROOT_DIR/build"

if [[ ! -f "$SRC_C" ]]; then
  echo "error: C source not found: $SRC_C" >&2
  exit 2
fi

if [[ ! -f "$INPUT_TXT" ]]; then
  cat > "$INPUT_TXT" <<'EOF'
put
k1
v1
put
k2
v2
get
k1
save
tmp_minidb_leakcheck.txt
exit
EOF
fi

echo "==> compile: $SRC_C"
gcc -g -O0 "$SRC_C" -o "$OUT_BIN"

echo "==> run valgrind"
valgrind \
  --leak-check=full \
  --show-leak-kinds=all \
  --track-origins=yes \
  --error-exitcode=101 \
  --log-file="$OUT_LOG" \
  "$OUT_BIN" < "$INPUT_TXT" >/dev/null

echo "==> valgrind finished: $OUT_LOG"
if grep -q "definitely lost: 0 bytes" "$OUT_LOG"; then
  echo "==> no definitely-lost leaks detected"
else
  echo "==> WARNING: definitely-lost leaks detected; inspect $OUT_LOG"
fi
