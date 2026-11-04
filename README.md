# Mozu

Mozu is a macOS app for voice typing and transcription on Apple Silicon. Transcription runs on device with [Mozu Audio](https://github.com/ooookao/mozu-audio).

- **Voice typing** — Use Fn to dictate into the focused app. Mozu keeps a copy in its history.
- **Transcription** — Open an audio file or record a microphone. Select **マイク＋システム音声** to also capture audio playing on the Mac. Live transcription appears as you record, and completed results are saved to history.
- **Dictionary** — Add names and terms with optional readings to help Mozu recognize them.

Download an encoder in **モデル** before starting. Grant microphone and system audio access when prompted, and enable Accessibility access to use Fn for voice typing. System audio capture requires macOS 14.2 or later.

## Development

Use Xcode 26 or later to compile the app icon. Use the same signing identity for each development build so macOS can retain permissions.

```sh
: "${MOZU_SIGNING_IDENTITY:?Set MOZU_SIGNING_IDENTITY to your code signing identity}"
cargo build --locked --target-dir target/dev-build
mozu_app="$HOME/Applications/Mozu Dev.app"
scripts/package-macos.sh target/dev-build/debug/mozu "$mozu_app"
plutil -replace CFBundleIdentifier -string local.mozu-audio.demo "$mozu_app/Contents/Info.plist"
plutil -replace CFBundleName -string "Mozu Dev" "$mozu_app/Contents/Info.plist"
plutil -insert CFBundleDisplayName -string "Mozu Dev" "$mozu_app/Contents/Info.plist"
codesign --force --sign "$MOZU_SIGNING_IDENTITY" "$mozu_app"
open "$mozu_app"
```

## License

Mozu is licensed under [AGPL-3.0-only](LICENSE).
