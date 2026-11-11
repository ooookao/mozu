#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cache="$repo_root/target/sparkle"
version="2.10.1"
checksum="1a856d2a9edb8e628de88121d7be04cf0e2167f8641650b78fca00b14244cf27"
archive="$cache/Sparkle-$version.tar.xz"
url="https://github.com/sparkle-project/Sparkle/releases/download/$version/Sparkle-$version.tar.xz"
mkdir -p "$cache"
download="$(mktemp "$cache/.download.XXXXXX")"
scratch="$(mktemp -d "$cache/.extract.XXXXXX")"
trap 'rm -f "$download"; rm -rf "$scratch"' EXIT

if [[ ! -f "$archive" ]]; then
    curl --fail --location --silent --show-error "$url" --output "$download"
    actual="$(shasum -a 256 "$download")"
    if [[ "${actual%% *}" != "$checksum" ]]; then
        echo "Sparkle checksum mismatch: url=$url expected=$checksum actual=${actual%% *}" >&2
        exit 1
    fi
    mv "$download" "$archive"
fi
actual="$(shasum -a 256 "$archive")"
if [[ "${actual%% *}" != "$checksum" ]]; then
    echo "Sparkle cache checksum mismatch: path=$archive expected=$checksum actual=${actual%% *}" >&2
    exit 1
fi

if [[ ! -f "$cache/.prepared-sha256" ]] \
    || [[ "$(cat "$cache/.prepared-sha256")" != "$checksum" ]] \
    || [[ ! -d "$cache/Sparkle.framework" ]] \
    || [[ ! -x "$cache/bin/generate_appcast" ]]; then
    tar -xf "$archive" -C "$scratch" ./Sparkle.framework ./bin ./LICENSE
    for item in Sparkle.framework bin LICENSE; do
        rm -rf "$cache/$item"
        ditto "$scratch/$item" "$cache/$item"
    done
    printf '%s\n' "$checksum" > "$cache/.prepared-sha256"
fi
printf '%s\n' "$cache"
