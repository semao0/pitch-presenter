#!/bin/sh
# Сборка .deb: ./build-deb.sh → dist/pitch-presenter_<ver>_all.deb
set -eu
VER=1.3.3
cd "$(dirname "$0")"
ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT

install -Dm644 present.py      "$ROOT/usr/lib/pitch-presenter/present.py"
install -Dm755 app.py          "$ROOT/usr/lib/pitch-presenter/app.py"
install -Dm644 web/presenter.html "$ROOT/usr/lib/pitch-presenter/web/presenter.html"
install -Dm644 web/audience.html  "$ROOT/usr/lib/pitch-presenter/web/audience.html"
install -Dm644 packaging/pitch-presenter.desktop "$ROOT/usr/share/applications/pitch-presenter.desktop"
install -Dm644 packaging/pitch-presenter.svg "$ROOT/usr/share/icons/hicolor/scalable/apps/pitch-presenter.svg"
mkdir -p "$ROOT/usr/bin"
ln -s ../lib/pitch-presenter/app.py "$ROOT/usr/bin/pitch-presenter"
printf '#!/bin/sh\nexec /usr/bin/python3 /usr/lib/pitch-presenter/present.py "$@"\n' > "$ROOT/usr/bin/pitch-presenter-cli"
chmod 755 "$ROOT/usr/bin/pitch-presenter-cli"

mkdir -p "$ROOT/DEBIAN"
cat > "$ROOT/DEBIAN/control" <<CTRL
Package: pitch-presenter
Version: $VER
Section: misc
Priority: optional
Architecture: all
Depends: python3 (>= 3.10), python3-gi, gir1.2-gtk-3.0, gir1.2-webkit2-4.1, poppler-utils
Suggests: libreoffice-impress
Installed-Size: $(du -sk "$ROOT/usr" | cut -f1)
Maintainer: semao0 <belousovsemen40@gmail.com>
Description: Presenter view for PDF/PPTX with speaker notes
 Opens a presenter window (current and next slide, speaker notes from
 Markdown, timer) and a separate audience window for screen sharing.
CTRL

mkdir -p dist
dpkg-deb --build --root-owner-group "$ROOT" "dist/pitch-presenter_${VER}_all.deb"
