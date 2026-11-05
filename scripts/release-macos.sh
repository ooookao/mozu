#!/bin/bash
set -euo pipefail

if [[ $# -ne 2 || ! "$1" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ || ! "$2" =~ ^[1-9][0-9]*$ ]]; then
    echo "Usage: $0 v<major.minor.patch> <macOS build number>" >&2
    echo "Set MOZU_SIGN_IDENTITY, APPLE_API_KEY_PATH, APPLE_API_KEY, and APPLE_API_ISSUER." >&2
    exit 2
fi

: "${MOZU_SIGN_IDENTITY:?Set the Developer ID Application signing identity}"
: "${APPLE_API_KEY_PATH:?Set the App Store Connect API key path}"
: "${APPLE_API_KEY:?Set the App Store Connect API key ID}"
: "${APPLE_API_ISSUER:?Set the App Store Connect issuer ID}"

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"
tag="$1"
build="$2"
tag_commit="$(git rev-parse --verify "refs/tags/$tag^{commit}")"
if [[ "$tag_commit" != "$(git rev-parse HEAD)" || -n "$(git status --porcelain)" ]]; then
    echo "Check out the release tag with a clean worktree." >&2
    exit 2
fi

out="$repo_root/target/releases/$tag"
if [[ -e "$out" ]]; then
    echo "Release output already exists: $out" >&2
    exit 2
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target}"
cargo test --locked
cargo build --release --locked
binary="$CARGO_TARGET_DIR/release/mozu"
version="$("$binary" --version)"
if [[ "$tag" != "v$version" ]]; then
    echo "Release tag $tag does not match app version $version." >&2
    exit 2
fi
lipo -verify_arch arm64 "$binary"

mkdir -p "$repo_root/target/releases"
stage="$(mktemp -d "$repo_root/target/releases/.build.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
app="$stage/image/Mozu.app"
name="Mozu-$version-macos-arm64"

notarize() {
    xcrun notarytool submit "$1" \
        --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" \
        --issuer "$APPLE_API_ISSUER" --wait --timeout 15m
}

scripts/package-macos.sh "$binary" "$app"
/usr/libexec/PlistBuddy -c "Set CFBundleVersion $build" "$app/Contents/Info.plist"
entitlements="$stage/Entitlements.plist"
plutil -create xml1 "$entitlements"
/usr/libexec/PlistBuddy -c "Add com.apple.security.device.audio-input bool true" "$entitlements"
codesign --force --options runtime --timestamp --entitlements "$entitlements" \
    --sign "$MOZU_SIGN_IDENTITY" "$app"
codesign --verify --deep --strict "$app"

ditto -c -k --keepParent "$app" "$stage/app-for-notarization.zip"
notarize "$stage/app-for-notarization.zip"
xcrun stapler staple "$app"
xcrun stapler validate "$app"
spctl --assess --type execute -vv "$app"

mkdir "$stage/release"
ditto -c -k --keepParent "$app" "$stage/release/$name.zip"
unzip -tq "$stage/release/$name.zip"
ln -s /Applications "$stage/image/Applications"
dmg="$stage/release/$name.dmg"
hdiutil create -quiet -srcfolder "$stage/image" -volname Mozu -format UDZO "$dmg"
codesign --timestamp --sign "$MOZU_SIGN_IDENTITY" "$dmg"
notarize "$dmg"
xcrun stapler staple "$dmg"
xcrun stapler validate "$dmg"
spctl --assess --type open --context context:primary-signature -vv "$dmg"
hdiutil verify -quiet "$dmg"

mv "$stage/release" "$out"
printf 'Release assets: %s\n' "$out"
