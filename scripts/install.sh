#!/usr/bin/env bash
# Install Zap on Linux (and other Unix) to ~/.local/bin.
# Prefers the GitHub release binary; builds from source if that asset is missing.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
mkdir -p "$BIN_DIR"
DEST="$BIN_DIR/zap"

asset=""
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64|Linux-amd64) asset="zap-linux-x86_64" ;;
esac

download_release() {
  local url="https://github.com/tobbikz/zap/releases/latest/download/${asset}"
  local tmp
  tmp="$(mktemp)"
  if curl -fsSL --retry 3 -o "$tmp" "$url"; then
    install -m 755 "$tmp" "$DEST"
    rm -f "$tmp"
    return 0
  fi
  rm -f "$tmp"
  return 1
}

if [[ -n "$asset" ]] && download_release; then
  echo "→ installed: $DEST (GitHub release)"
else
  echo "→ release binary unavailable, building from source…"
  cd "$ROOT"
  cargo build -p ide-app --release
  install -m 755 "$ROOT/target/release/zap" "$DEST"
  echo "→ installed: $DEST"
fi

if ! command -v zap >/dev/null 2>&1; then
  echo "  note: add to PATH if needed:"
  echo "    export PATH=\"$BIN_DIR:\$PATH\""
fi
"$DEST" --version 2>/dev/null || ls -lh "$DEST"
echo "done."
