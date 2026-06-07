#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/ittsy}"
destination="${2:-dist}"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
app="${destination}/ittsy.app"

test -x "${binary}" || {
  echo "Missing executable: ${binary}" >&2
  exit 1
}

rm -rf "${app}"
mkdir -p "${app}/Contents/MacOS" "${app}/Contents/Resources"
cp "${binary}" "${app}/Contents/MacOS/ittsy"

cat > "${app}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>ittsy</string>
  <key>CFBundleExecutable</key>
  <string>ittsy</string>
  <key>CFBundleIdentifier</key>
  <string>design.resist.ittsy</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>ittsy</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${version}</string>
  <key>CFBundleVersion</key>
  <string>${version}</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

plutil -lint "${app}/Contents/Info.plist"
codesign --force --sign - --timestamp=none "${app}"
echo "${app}"
