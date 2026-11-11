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
sparkle_dir="$(scripts/prepare-sparkle.sh)"
export DYLD_FRAMEWORK_PATH="$sparkle_dir"
sparkle_account="io.github.ooookao.mozu"
public_key="$("$sparkle_dir/bin/generate_keys" --account "$sparkle_account" -p)"
expected_key="$(/usr/libexec/PlistBuddy -c 'Print SUPublicEDKey' Info.plist)"
if [[ "$public_key" != "$expected_key" ]]; then
    echo "The Sparkle signing key does not match SUPublicEDKey in Info.plist." >&2
    exit 1
fi
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
scripts/sign-macos.sh "$app" "$MOZU_SIGN_IDENTITY" "$entitlements"

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

mkdir "$stage/feed"
cp "$stage/release/$name.zip" "$stage/feed/"
"$sparkle_dir/bin/generate_appcast" \
    --account "$sparkle_account" \
    --download-url-prefix "https://github.com/ooookao/mozu/releases/download/$tag/" \
    --maximum-deltas 0 --disable-signing-warning "$stage/feed"
cp "$stage/feed/appcast.xml" "$stage/release/appcast.xml"
"$sparkle_dir/bin/sign_update" --account "$sparkle_account" --verify "$stage/release/appcast.xml"

mv "$stage/release" "$out"
printf 'Release assets: %s\n' "$out"
