#!/bin/sh
# Downloads a prebuilt PDFium (https://github.com/bblanchon/pdfium-binaries)
# into src-tauri/pdfium/, where both `tauri dev` and the bundler pick it up.
#
#   scripts/fetch-pdfium.sh            # library for the current machine
#   scripts/fetch-pdfium.sh mac-arm64  # explicit target: linux-x64, linux-arm64, mac-arm64, mac-x64, win-x64
set -eu

RELEASE="chromium/8111"
cd "$(dirname "$0")/.."

if [ $# -gt 0 ]; then
  target=$1
else
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) target=linux-x64 ;;
    Linux-aarch64) target=linux-arm64 ;;
    Darwin-arm64) target=mac-arm64 ;;
    Darwin-x86_64) target=mac-x64 ;;
    *) echo "unsupported platform: $(uname -sm)" >&2; exit 1 ;;
  esac
fi

case "$target" in
  linux-*) lib=lib/libpdfium.so ;;
  mac-*) lib=lib/libpdfium.dylib ;;
  win-*) lib=bin/pdfium.dll ;;
  *) echo "unknown target: $target" >&2; exit 1 ;;
esac

out=src-tauri/pdfium
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

url="https://github.com/bblanchon/pdfium-binaries/releases/download/$RELEASE/pdfium-$target.tgz"
echo "fetching $url"
curl -fsSL "$url" | tar -xz -C "$tmp"
mkdir -p "$out"
cp "$tmp/$lib" "$out/"
cp "$tmp/LICENSE" "$out/LICENSE.pdfium"
# PDFium and the libraries built into it (FreeType, ICU, libpng…) ship with the app.
rm -rf "$out/licenses"
cp -R "$tmp/licenses" "$out/licenses"
cp "$tmp/LICENSE" "$out/licenses/pdfium-binaries.txt"
echo "installed $out/$(basename "$lib")"
