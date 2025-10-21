#!/usr/bin/env bash

set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mkdir -p "$BIN_DIR"

cp "$REPO_ROOT/tools/axion_runner.py" "$BIN_DIR/axion_runner.py"
cp "$REPO_ROOT/tools/axion" "$BIN_DIR/axion"
chmod +x "$BIN_DIR/axion" "$BIN_DIR/axion_runner.py"

echo "[axion] Installed runner to $BIN_DIR"
echo "[axion] Ensure $BIN_DIR is on your PATH (e.g., export PATH=\"$BIN_DIR:\$PATH\")"
