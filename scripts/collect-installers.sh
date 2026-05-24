#!/usr/bin/env bash
# Collect platform installer artifacts after `npm run tauri build`.
# Avoids uploading bundle internals (AppImage rootfs, etc.).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/release-installers"
TARGET="$ROOT/src-tauri/target"

mkdir -p "$OUT"

copy_installer() {
  local f="$1"
  cp "$f" "$OUT/$(basename "$f")"
}

find_macos_app() {
  find "$TARGET" -type d -name '*.app' -path '*/bundle/macos/*' 2>/dev/null | head -1 || true
}

# Top-level bundle outputs only (not nested AppImage payloads).
while IFS= read -r -d '' f; do
  case "$f" in
    */bundle/dmg/*|*/bundle/msi/*|*/bundle/nsis/*|*/bundle/deb/*|*/bundle/appimage/*|*/bundle/rpm/*)
      copy_installer "$f"
      ;;
  esac
done < <(
  find "$TARGET" -type f \( \
    -name '*.dmg' -o \
    -name '*.msi' -o \
    -name '*.deb' -o \
    -name '*.rpm' -o \
    -name '*.AppImage' -o \
    -name '*-setup.exe' \
  \) -print0 2>/dev/null
)

# macOS: DMG creation can fail in CI while the .app bundle still builds.
shopt -s nullglob
dmgs=("$OUT"/*.dmg)
if ((${#dmgs[@]} == 0)) && [[ "$(uname -s)" == "Darwin" ]]; then
  app_path="$(find_macos_app)"
  if [[ -n "$app_path" ]]; then
    zip_name="$(basename "$app_path").zip"
    echo "No .dmg found; zipping $app_path -> $zip_name"
    ditto -c -k --keepParent "$app_path" "$OUT/$zip_name"
  fi
fi

if [[ -z "$(ls -A "$OUT" 2>/dev/null)" ]]; then
  echo "::error::No installer files found under $TARGET/.../bundle/"
  find "$TARGET" -type d -name bundle 2>/dev/null | head -10 || true
  find "$TARGET" -path '*/bundle/*' -maxdepth 5 \( -type f -o -type d \) 2>/dev/null | head -80 || true
  if [[ "$(uname -s)" == "Darwin" ]]; then
    echo "macOS .app search:"
    find_macos_app || true
  fi
  exit 1
fi

ls -la "$OUT"
