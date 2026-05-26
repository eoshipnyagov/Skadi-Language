#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

INSTALL_ROOT="${1:-${XDG_DATA_HOME:-$HOME/.local/share}/skadi}"
BIN_DIR="$INSTALL_ROOT/bin"

echo "==> Installing skadi-cli to: $INSTALL_ROOT"
cargo install --path tools/skadi-cli --root "$INSTALL_ROOT" --force

SKADI_WRAPPER="$BIN_DIR/skadi"
cat > "$SKADI_WRAPPER" <<'EOF'
#!/usr/bin/env bash
exec "$(dirname "$0")/skadi-cli" "$@"
EOF
chmod +x "$SKADI_WRAPPER"

if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
  SHELL_RC="$HOME/.bashrc"
  if [[ "${SHELL##*/}" == "zsh" ]]; then
    SHELL_RC="$HOME/.zshrc"
  fi
  echo "==> PATH does not include $BIN_DIR"
  echo "Add this line to $SHELL_RC:"
  echo "export PATH=\"$BIN_DIR:\$PATH\""
else
  echo "==> PATH already contains: $BIN_DIR"
fi

echo "==> Verifying skadi command..."
"$SKADI_WRAPPER" help

echo
echo "Install complete."
