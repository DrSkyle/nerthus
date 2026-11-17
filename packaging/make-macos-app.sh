#!/bin/bash
# Wraps the nerthus binary into a minimal but valid macOS .app bundle.
# Usage: make-macos-app.sh <binary> <output-dir>
set -euo pipefail

BIN="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
OUT="$2"
APP="$OUT/Nerthus.app"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BIN" "$APP/Contents/MacOS/nerthus"
chmod +x "$APP/Contents/MacOS/nerthus"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Nerthus</string>
    <key>CFBundleDisplayName</key>
    <string>Nerthus</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.drskyle.nerthus</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>nerthus</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

# Ad-hoc sign so Gatekeeper accepts it locally without a developer certificate.
codesign --force --deep --sign - "$APP" 2>/dev/null || echo "codesign skipped (not available)"

echo "built $APP"
