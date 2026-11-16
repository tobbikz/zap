#!/usr/bin/env bash
# Install Zap on Linux (and other Unix) to ~/.local/bin
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
mkdir -p "$BIN_DIR"

echo "→ building release…"
cargo build -p ide-app --release

SRC="$ROOT/target/release/zap"
install -m 755 "$SRC" "$BIN_DIR/zap"

echo "→ installed: $BIN_DIR/zap"
if ! command -v zap >/dev/null 2>&1; then
  echo "  note: add to PATH if needed:"
  echo "    export PATH=\"$BIN_DIR:\$PATH\""
fi
"$BIN_DIR/zap" --version 2>/dev/null || ls -lh "$BIN_DIR/zap"
echo "done."
