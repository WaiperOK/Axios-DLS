#!/usr/bin/env bash
set -euo pipefail

VERSION="${VERSION:-dev}"
CONFIGURATION="${CONFIGURATION:-release}"
TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
OUTPUT_DIR="${OUTPUT_DIR:-build/package/linux}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "[package] building axion-cli ($CONFIGURATION, $TARGET)"
cargo build --"$CONFIGURATION" -p axion-cli --target "$TARGET" >/dev/null

STAGE_ROOT="$REPO_ROOT/build/stage/linux-$VERSION"
rm -rf "$STAGE_ROOT"
mkdir -p "$STAGE_ROOT/bin"

BINARY_PATH="$REPO_ROOT/target/$TARGET/$CONFIGURATION/axion-cli"
if [[ ! -f "$BINARY_PATH" ]]; then
  echo "[package] missing binary: $BINARY_PATH" >&2
  exit 1
fi
cp "$BINARY_PATH" "$STAGE_ROOT/bin/axion"

for dir in examples docs tools ui/react-flow-prototype/dist; do
  if [[ -d "$REPO_ROOT/$dir" ]]; then
    mkdir -p "$(dirname "$STAGE_ROOT/$dir")"
    cp -R "$REPO_ROOT/$dir" "$STAGE_ROOT/$dir"
  else
    echo "[package] warning: directory not found: $dir"
  fi
done

for file in LICENSE README.md; do
  if [[ -f "$REPO_ROOT/$file" ]]; then
    cp "$REPO_ROOT/$file" "$STAGE_ROOT/$file"
  fi
done

cat >"$STAGE_ROOT/install.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

PREFIX="${PREFIX:-/usr/local}"
BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/axion"

mkdir -p "$BIN_DIR" "$SHARE_DIR"
cp "./bin/axion" "$BIN_DIR/axion"
cp -R ./examples "$SHARE_DIR/examples"
cp -R ./tools "$SHARE_DIR/tools"

echo "[axion] Installed to $PREFIX (binary: $BIN_DIR/axion)"
echo "[axion] Ensure $BIN_DIR is on your PATH."
EOF
chmod +x "$STAGE_ROOT/install.sh"

mkdir -p "$OUTPUT_DIR"
ARCHIVE_PATH="$OUTPUT_DIR/axion-$VERSION-linux-x64.tar.gz"
tar -C "$STAGE_ROOT" -czf "$ARCHIVE_PATH" .

echo "[package] archive created at $ARCHIVE_PATH"
