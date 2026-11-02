#!/usr/bin/env bash
# Builds a release binary and wraps it in Swatchbar.app (menu-bar only, no Dock icon).
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release
OUT="${1:-target/Swatchbar.app}"
BIN="${CARGO_TARGET_DIR:-target}/release/swatchbar"
rm -rf "$OUT"
mkdir -p "$OUT/Contents/MacOS"
cp "$BIN" "$OUT/Contents/MacOS/Swatchbar"
mkdir -p "$OUT/Contents/Resources"
cp assets/Swatchbar.icns "$OUT/Contents/Resources/Swatchbar.icns"
cat > "$OUT/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Swatchbar</string>
  <key>CFBundleDisplayName</key><string>Swatchbar</string>
  <key>CFBundleIdentifier</key><string>com.swatchbar.app</string>
  <key>CFBundleExecutable</key><string>Swatchbar</string>
  <key>CFBundleIconFile</key><string>Swatchbar</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSScreenCaptureUsageDescription</key><string>Swatchbar reads the pixel under your cursor to sample colors.</string>
</dict></plist>
PLIST
# Prefer the stable "Swatchbar Dev" identity (see make_cert.sh); fall back to ad-hoc.
IDENT="-"; security find-identity -p codesigning 2>/dev/null | grep -q "Swatchbar Dev" && IDENT="Swatchbar Dev"
codesign --force --timestamp=none --sign "$IDENT" "$OUT" >/dev/null 2>&1 || true
echo "Signed with: $IDENT"
echo "Built $OUT"
