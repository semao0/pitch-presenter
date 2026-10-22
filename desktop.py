#!/usr/bin/env python3
"""Pitch Presenter — окно на pywebview (macOS: WKWebView). Linux-версия — app.py (GTK)."""
import json
import os
import subprocess
import sys
import threading
from pathlib import Path
from types import SimpleNamespace

import webview

import present

NAME = "Pitch Presenter"
PORT = 47615  # постоянный порт — чтобы шрифт (localStorage) не терялся между запусками
if sys.platform == "darwin":
    DATA = Path.home() / "Library" / "Application Support" / NAME
else:
    DATA = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "pitch-presenter"
CONFIG = DATA / "config.json"
WEB = Path(getattr(sys, "_MEIPASS", Path(__file__).resolve().parent)) / "web"

DECK_TYPES = ("Презентации (*.pdf;*.pptx;*.ppt;*.odp;*.key)",)
NOTE_TYPES = ("Текст спикера (*.toml;*.md;*.markdown;*.txt)",)
NOTE_EXT = (".toml", ".md", ".markdown", ".txt")


def load_config():
    try:
        return json.loads(CONFIG.read_text())
    except Exception:
        return {}


def save_config(cfg):
    try:
        CONFIG.parent.mkdir(parents=True, exist_ok=True)
        CONFIG.write_text(json.dumps(cfg, ensure_ascii=False, indent=1))
    except OSError:
        pass


def guess_notes(deck: Path):
    return next((str(p) for p in map(deck.with_suffix, NOTE_EXT) if p.exists()), None)


class Api:
    """Методы доступны из страниц как pywebview.api.*; поля с «_» наружу не торчат."""

    def __init__(self, deck=None, notes=None):
        self._main = self._audience = self._srv = None
        self._url = ""
        self._files = (deck, notes)

    def init(self):
        cfg = load_config()
        deck, notes = self._files
        deck = deck or cfg.get("deck")
        if not (deck and Path(deck).exists()):
            return {"deck": None, "notes": None, "minutes": cfg.get("minutes", 0)}
        notes = notes or (cfg.get("notes") if deck == cfg.get("deck") else None) or guess_notes(Path(deck))
        if notes and not Path(notes).exists():
            notes = None
        return {"deck": deck, "notes": notes, "minutes": cfg.get("minutes", 0)}

    def pick(self, kind):
        types = DECK_TYPES if kind == "deck" else NOTE_TYPES
        res = self._main.create_file_dialog(webview.FileDialog.OPEN, file_types=types)
        if not res:
            return None
        path = res[0] if isinstance(res, (list, tuple)) else res
        return {"path": path, "notes": guess_notes(Path(path)) if kind == "deck" else None}

    def start(self, deck, notes, minutes):
        if not deck or not Path(deck).exists():
            return {"error": "Выберите файл презентации"}
        minutes = float(minutes or 0)
        save_config({"deck": deck, "notes": notes, "minutes": minutes})
        args = SimpleNamespace(deck=Path(deck), minutes=minutes)
        notes_path = Path(notes) if notes else None
        try:
            slides = present.render_slides(args.deck, 1920)
            if not slides:
                raise RuntimeError("Не удалось получить слайды")
            present.parse_notes(notes_path)  # битый текст — ошибка сразу, а не пустой экран
            srv, port = present.serve(slides, notes_path, args, PORT)
        except (RuntimeError, OSError, ValueError, subprocess.CalledProcessError) as e:
            return {"error": str(e)}
        threading.Thread(target=srv.serve_forever, daemon=True).start()
        self._srv, self._url = srv, f"http://127.0.0.1:{port}/"
        self._main.load_url(self._url)
        self._main.maximize()
        return {"ok": True}

    def open_audience(self):
        if self._audience:
            self._audience.show()
            return
        win = webview.create_window("Показ", self._url + "audience", js_api=self, width=1280, height=720,
                                    background_color="#000000")
        win.events.closed += self._audience_closed
        win.events.shown += lambda: mac_no_throttle(win)
        self._audience = win

    def _audience_closed(self):
        self._audience = None

    def fullscreen(self):
        if self._audience:
            self._audience.toggle_fullscreen()

    def _quit(self):
        if self._audience:
            self._audience.destroy()
        if self._srv:
            self._srv.bus.close()
            self._srv.shutdown()


def mac_no_throttle(win):
    """macOS глушит перекрытые и фоновые окна: WebKit перестаёт рисовать и душит таймеры,
    App Nap усыпляет процесс. Для окна показа, которое шарится в созвон, это секунды задержки."""
    if sys.platform != "darwin":
        return
    try:
        from Foundation import NSProcessInfo
        from PyObjCTools import AppHelper
        from webview.platforms.cocoa import BrowserView

        info = NSProcessInfo.processInfo()
        # NSActivityUserInitiated | NSActivityLatencyCritical — без App Nap, пока приложение открыто
        mac_no_throttle.activity = info.beginActivityWithOptions_reason_(0x00FFFFFF | 0xFF00000000, "Показ слайдов")

        def tweak():
            view = BrowserView.instances[win.uid].webview
            if view.respondsToSelector_("_setWindowOcclusionDetectionEnabled:"):
                view._setWindowOcclusionDetectionEnabled_(False)

        AppHelper.callAfter(tweak)
    except Exception as e:  # приватный API может пропасть — показ всё равно работает
        print("mac_no_throttle:", e, file=sys.stderr)


def main():
    files = [str(Path(a).resolve()) for a in sys.argv[1:] if not a.startswith("-")]
    deck = next((f for f in files if not f.lower().endswith(NOTE_EXT)), None)
    notes = next((f for f in files if f.lower().endswith(NOTE_EXT)), None)
    api = Api(deck, notes)
    win = webview.create_window(NAME, html=(WEB / "start.html").read_text(encoding="utf-8"), js_api=api,
                                width=620, height=460, min_size=(520, 420), background_color="#14161a")
    api._main = win
    win.events.shown += lambda: mac_no_throttle(win)
    win.events.closed += api._quit
    DATA.mkdir(parents=True, exist_ok=True)
    webview.start(private_mode=False, storage_path=str(DATA / "web"), debug="--debug" in sys.argv)


if __name__ == "__main__":
    main()
