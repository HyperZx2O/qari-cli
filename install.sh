#!/usr/bin/env bash
set -euo pipefail

REPO="HyperZx2O/qari-cli"
BINARY="qari"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

case "$OS" in
  linux) TARGET="${ARCH}-unknown-linux-gnu" ;;
  darwin) TARGET="${ARCH}-apple-darwin" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

LATEST="${1:-${VERSION:-}}"
if [ -z "$LATEST" ]; then
  LATEST=$(curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
    --max-time 30 --retry 2 \
    "https://api.github.com/repos/${REPO}/releases/latest" \
    | sed -n 's/.*"tag_name": "\([^"]*\)".*/\1/p' \
    | head -1)
fi

if ! [[ "$LATEST" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$ ]]; then
  echo "A valid versioned release tag is required" >&2
  exit 1
fi

URL="https://github.com/${REPO}/releases/download/${LATEST}/qari-${TARGET}.tar.gz"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
ARCHIVE_NAME="qari-${TARGET}.tar.gz"
MAX_DOWNLOAD_BYTES=$((128 * 1024 * 1024))
MAX_CHECKSUM_BYTES=$((1024 * 1024))

command -v curl >/dev/null || { echo "curl is required" >&2; exit 1; }
command -v sha256sum >/dev/null || { echo "sha256sum is required" >&2; exit 1; }
command -v tar >/dev/null || { echo "tar is required" >&2; exit 1; }

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
ARCHIVE="$TMP_DIR/$ARCHIVE_NAME"
SUMS="$TMP_DIR/SHA256SUMS.txt"

ASSET_URL=$(curl --proto '=https' --proto-redir '=https' --tlsv1.2 \
  --fail --silent --show-error --location --max-redirs 5 \
  --max-time 60 --retry 2 --max-filesize "$MAX_DOWNLOAD_BYTES" \
  --output "$ARCHIVE" --write-out '%{url_effective}' "$URL")
case "$ASSET_URL" in
  https://github.com/*|https://release-assets.githubusercontent.com/*|https://objects.githubusercontent.com/*) ;;
  *) echo "Refusing unexpected release asset host" >&2; exit 1 ;;
esac

CHECKSUM_URL=$(curl --proto '=https' --proto-redir '=https' --tlsv1.2 \
  --fail --silent --show-error --location --max-redirs 5 \
  --max-time 30 --retry 2 --max-filesize "$MAX_CHECKSUM_BYTES" \
  --output "$SUMS" --write-out '%{url_effective}' \
  "https://github.com/${REPO}/releases/download/${LATEST}/SHA256SUMS.txt")
case "$CHECKSUM_URL" in
  https://github.com/*|https://release-assets.githubusercontent.com/*|https://objects.githubusercontent.com/*) ;;
  *) echo "Refusing unexpected checksum host" >&2; exit 1 ;;
esac

[ "$(wc -c < "$ARCHIVE")" -le "$MAX_DOWNLOAD_BYTES" ] || {
  echo "Release archive exceeds the size limit" >&2
  exit 1
}
[ "$(wc -c < "$SUMS")" -le "$MAX_CHECKSUM_BYTES" ] || {
  echo "Checksum file exceeds the size limit" >&2
  exit 1
}

EXPECTED=$(awk -v file="$ARCHIVE_NAME" '$2 == file || $2 == "*" file { print $1 }' "$SUMS")
if ! [[ "$EXPECTED" =~ ^[0-9a-fA-F]{64}$ ]]; then
  echo "No valid checksum found for $ARCHIVE_NAME" >&2
  exit 1
fi
ACTUAL=$(sha256sum "$ARCHIVE" | awk '{ print $1 }')
[ "$EXPECTED" = "$ACTUAL" ] || {
  echo "Checksum mismatch for $ARCHIVE_NAME" >&2
  exit 1
}

MEMBERS=$(tar -tzf "$ARCHIVE")
[ "$MEMBERS" = "$BINARY" ] || {
  echo "Archive contains unexpected members" >&2
  exit 1
}
LISTING=$(tar -tvzf "$ARCHIVE")
case "$LISTING" in
  -*) ;;
  *) echo "Archive binary is not a regular file" >&2; exit 1 ;;
esac

tar --no-same-owner --no-same-permissions -xzf "$ARCHIVE" -C "$TMP_DIR"
[ -f "$TMP_DIR/$BINARY" ] || { echo "Archive binary is missing" >&2; exit 1; }
mkdir -p "$INSTALL_DIR"
install -m 755 "$TMP_DIR/$BINARY" "$INSTALL_DIR/$BINARY"
echo "Done. Run: $INSTALL_DIR/$BINARY"
