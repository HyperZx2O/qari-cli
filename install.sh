#!/usr/bin/env bash
set -euo pipefail

REPO="HyperZx2O/qari-cli"
BINARY="islam"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

case "$OS" in
  linux) TARGET="${ARCH}-unknown-linux-gnu" ;;
  darwin) TARGET="${ARCH}-apple-darwin" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | sed -n 's/.*"tag_name": "\([^"]*\)".*/\1/p' | head -1)
URL="https://github.com/${REPO}/releases/download/${LATEST}/islam-${TARGET}.tar.gz"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

echo "Downloading islam-cli ${LATEST} for ${TARGET}..."
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
curl -fsSL "$URL" | tar -xz -C "$TMP_DIR"
install -m 755 "$TMP_DIR/$BINARY" "$INSTALL_DIR/$BINARY"
echo "Done. Run: islam"
