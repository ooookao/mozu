#!/bin/bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <mozu-binary> <output.app>" >&2
    exit 2
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
binary="$1"
bundle="$2"
if [[ ! -x "$binary" || "$bundle" != *.app ]]; then
    echo "Expected an executable Mozu binary and a path ending in .app" >&2
    exit 2
fi

sparkle_dir="$("$repo_root/scripts/prepare-sparkle.sh")"
export DYLD_FRAMEWORK_PATH="$sparkle_dir"

icon_build="$(mktemp -d)"
trap 'rm -rf "$icon_build"' EXIT
mkdir "$icon_build/resources"
xcrun actool "$repo_root/assets/AppIcon.icon" \
    --compile "$icon_build/resources" \
    --platform macosx \
    --minimum-deployment-target 14.2 \
    --app-icon AppIcon \
    --output-partial-info-plist "$icon_build/Info.plist"

install -d "$bundle/Contents/MacOS" "$bundle/Contents/Resources" "$bundle/Contents/Frameworks"
ditto "$sparkle_dir/Sparkle.framework" "$bundle/Contents/Frameworks/Sparkle.framework"
install -m 644 "$sparkle_dir/LICENSE" "$bundle/Contents/Resources/Sparkle-LICENSE"
install -m 644 "$repo_root/assets/licenses/sparkle-updater.txt" "$bundle/Contents/Resources/sparkle-updater-LICENSE"
install -m 755 "$binary" "$bundle/Contents/MacOS/mozu"
install -m 644 "$repo_root/Info.plist" "$bundle/Contents/Info.plist"
version="$("$binary" --version)"
/usr/libexec/PlistBuddy -c "Add CFBundleShortVersionString string $version" "$bundle/Contents/Info.plist"
install -m 644 "$icon_build/resources/AppIcon.icns" "$icon_build/resources/Assets.car" \
    "$bundle/Contents/Resources/"
