#!/bin/bash
set -euo pipefail

if [[ $# -ne 3 || "$1" != *.app ]]; then
    echo "Usage: $0 <app-bundle> <signing-identity> <entitlements-plist>" >&2
    exit 2
fi
app="$1"
identity="$2"
entitlements="$3"
framework="$app/Contents/Frameworks/Sparkle.framework"
for helper in \
    "$framework/Versions/B/XPCServices/Downloader.xpc" \
    "$framework/Versions/B/XPCServices/Installer.xpc" \
    "$framework/Versions/B/Autoupdate" \
    "$framework/Versions/B/Updater.app" \
    "$framework"; do
    codesign --force --options runtime --timestamp --preserve-metadata=entitlements \
        --sign "$identity" "$helper"
done
codesign --force --options runtime --timestamp --entitlements "$entitlements" \
    --sign "$identity" "$app"
codesign --verify --deep --strict "$app"
