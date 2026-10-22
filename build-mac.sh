#!/usr/bin/env bash
# Сборка Pitch Presenter.app и .dmg — запускать на Mac (Python 3.11+).
set -euo pipefail
cd "$(dirname "$0")"
NAME="Pitch Presenter"
VER=1.3.1
ARCH=$(uname -m)
PY=${PYTHON:-python3}

$PY -m venv .venv-mac
. .venv-mac/bin/activate
pip install -q --upgrade pip
pip install -q -r requirements-mac.txt

rm -rf build "dist/$NAME.app"
pyinstaller --noconfirm --windowed --name "$NAME" \
  --icon packaging/PitchPresenter.icns \
  --osx-bundle-identifier dev.semao0.PitchPresenter \
  --add-data "web:web" \
  desktop.py

APP="dist/$NAME.app"
PL="$APP/Contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$VER" "$PL"
plutil -replace NSHighResolutionCapable -bool true "$PL"
plutil -replace LSMinimumSystemVersion -string "11.0" "$PL"
# окна грузят страницы с локального http-сервера — разрешаем это явно
plutil -replace NSAppTransportSecurity -json '{"NSAllowsLocalNetworking":true,"NSAllowsArbitraryLoadsInWebContent":true}' "$PL"
codesign --force --deep --sign - "$APP"   # ad-hoc подпись, иначе на Apple Silicon не запустится

STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
DMG="dist/PitchPresenter-$VER-$ARCH.dmg"
rm -f "$DMG"
for i in 1 2 3 4 5; do  # на CI hdiutil иногда падает с «Resource busy»
  hdiutil create -volname "$NAME" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null && break
  [ "$i" = 5 ] && exit 1
  sleep 5
done
rm -rf "$STAGE"
echo "Готово: $DMG"
