#!/usr/bin/env bash
# Builds a release binary and packs it with a .desktop file and icon into a tarball.
# Build deps (Debian/Ubuntu): libgtk-3-dev libayatana-appindicator3-dev libxdo-dev
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release
ARCH="$(uname -m)"
OUT="${1:-target/swatchbar-linux-$ARCH}"
BIN="${CARGO_TARGET_DIR:-target}/release/swatchbar"
rm -rf "$OUT" "$OUT.tar.gz"
mkdir -p "$OUT"
cp "$BIN" "$OUT/swatchbar"
cp packaging/linux/swatchbar.desktop "$OUT/"
cp assets/Swatchbar.iconset/icon_256x256.png "$OUT/swatchbar.png"
tar -C "$(dirname "$OUT")" -czf "$OUT.tar.gz" "$(basename "$OUT")"
echo "Built $OUT.tar.gz"
